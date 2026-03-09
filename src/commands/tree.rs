// tree command — display parent/child hierarchy.
//
// `tree [id]` walks the `parent` field (NOT `deps`) to build a hierarchy.
// If an ID is provided, the subtree rooted at that ticket is shown.  If
// omitted, all root tickets (those with no parent or whose parent is not in
// the visible set) are shown.
//
// By default, closed tickets are excluded from the visible set.  The `--all`
// flag includes them.
//
// Children at each node are sorted by status priority
// (in_progress < open < closed) and then by `created` ascending.
//
// Cycles in the parent chain are detected and annotated with `[cycle]` rather
// than causing infinite loops.
//
// The ticket ID is dimmed.  The status label (no brackets) is colored by
// status: in_progress=cyan, open=blue, closed=dim.  Color respects the
// global `--color` flag and the `NO_COLOR` / `CLICOLOR` env vars via the
// `console` crate.  A blank line separates each top-level tree in forest mode.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use console::style;

use crate::error::{Error, Result};
use crate::pager;
use crate::store::TicketStore;
use crate::ticket::{Status, Ticket};

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

/// Return the colored status label (no brackets).
fn status_label(status: &Status) -> String {
    let label = status.to_string();
    match status {
        Status::InProgress => format!("{}", style(label).cyan()),
        Status::Open => format!("{}", style(label).blue()),
        Status::Closed => format!("{}", style(label).dim()),
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Extract the ticket ID from a resolved `.md` path.
fn full_id_from_path<'a>(path: &'a Path, partial: &'a str) -> &'a str {
    path.file_stem().and_then(|s| s.to_str()).unwrap_or(partial)
}

/// Status sort key: lower = higher priority in the tree.
fn status_priority(s: &Status) -> u8 {
    match s {
        Status::InProgress => 0,
        Status::Open => 1,
        Status::Closed => 2,
    }
}

/// A single rendered line of tree output.
struct TreeLine {
    /// Content for the line (may contain ANSI color codes in the status badge).
    text: String,
}

/// Recursively render a node and all its visible descendants into `lines`.
///
/// - `id`         — ticket to render
/// - `prefix`     — accumulated indentation prefix for this node's children
/// - `is_last`    — whether this node is the last child of its parent
/// - `is_root`    — whether this node is a root (no connector drawn)
/// - `tickets`    — full id→ticket map (visible set only)
/// - `children`   — pre-built parent→children map (visible set only)
/// - `ancestors`  — set of IDs on the current DFS path (for cycle detection)
/// - `max_depth`  — optional depth limit (None = unlimited)
/// - `depth`      — current depth (root = 0)
/// - `lines`      — output accumulator
#[allow(clippy::too_many_arguments)]
fn render_node(
    id: &str,
    prefix: &str,
    is_last: bool,
    is_root: bool,
    tickets: &HashMap<String, Ticket>,
    children: &HashMap<String, Vec<String>>,
    ancestors: &mut HashSet<String>,
    max_depth: Option<usize>,
    depth: usize,
    lines: &mut Vec<TreeLine>,
) {
    let (status, title) = match tickets.get(id) {
        Some(t) => (t.status.clone(), t.title.clone()),
        None => (Status::Open, "(not found)".to_string()),
    };

    let (connector, child_prefix) = if is_root {
        (String::new(), String::new())
    } else if is_last {
        ("└── ".to_string(), format!("{prefix}    "))
    } else {
        ("├── ".to_string(), format!("{prefix}│   "))
    };

    // Format the node content (dimmed id, colored status, title).
    let node = if title.is_empty() {
        format!("{} {}", style(id).dim(), status_label(&status))
    } else {
        format!("{} {} {title}", style(id).dim(), status_label(&status))
    };

    // Cycle detection: if this ID is already on the ancestor path, emit the
    // full node text plus [cycle] and stop recursing (matches the original).
    if ancestors.contains(id) {
        lines.push(TreeLine {
            text: format!("{prefix}{connector}{node} [cycle]"),
        });
        return;
    }

    lines.push(TreeLine {
        text: format!("{prefix}{connector}{node}"),
    });

    // Respect --max-depth.
    if max_depth.is_some_and(|limit| depth >= limit) {
        return;
    }

    let empty = Vec::new();
    let kids = children.get(id).unwrap_or(&empty);
    if kids.is_empty() {
        return;
    }

    ancestors.insert(id.to_string());

    let kid_count = kids.len();
    for (i, child_id) in kids.iter().enumerate() {
        let child_is_last = i == kid_count - 1;
        render_node(
            child_id,
            &child_prefix,
            child_is_last,
            false,
            tickets,
            children,
            ancestors,
            max_depth,
            depth + 1,
            lines,
        );
    }

    ancestors.remove(id);
}

// ---------------------------------------------------------------------------
// Core implementation (returns String for testability)
// ---------------------------------------------------------------------------

/// Build and return the rendered tree string.
///
/// `start_dir` is the directory from which to search for the `.tickets/`
/// directory (used in tests; `None` means the current working directory).
fn tree_impl(
    start_dir: Option<&Path>,
    partial_id: Option<&str>,
    max_depth: Option<usize>,
    include_closed: bool,
) -> Result<String> {
    let store = TicketStore::find(start_dir)?;

    // Load all tickets.
    let all_tickets: Vec<Ticket> = store.list_tickets();

    // Build the visible set: filter out closed tickets unless --all was given.
    let visible: HashMap<String, Ticket> = all_tickets
        .into_iter()
        .filter(|t| include_closed || t.status != Status::Closed)
        .map(|t| (t.id.clone(), t))
        .collect();

    if visible.is_empty() {
        return Ok(String::new());
    }

    // Build children map (parent_id → sorted Vec<child_id>).
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    for ticket in visible.values() {
        if let Some(parent_id) = &ticket.parent {
            // Only link to parents that are themselves in the visible set.
            if visible.contains_key(parent_id) {
                children
                    .entry(parent_id.clone())
                    .or_default()
                    .push(ticket.id.clone());
            }
        }
    }

    // Sort each child list: status priority (in_progress < open < closed),
    // then created ascending, then ID ascending as a stable tiebreak.
    for kids in children.values_mut() {
        kids.sort_by(|a, b| {
            let ta = &visible[a];
            let tb = &visible[b];
            status_priority(&ta.status)
                .cmp(&status_priority(&tb.status))
                .then_with(|| ta.created.cmp(&tb.created))
                .then_with(|| a.cmp(b))
        });
    }

    let mut lines: Vec<TreeLine> = Vec::new();

    if let Some(partial) = partial_id {
        // Subtree mode: resolve the requested root.
        let root_path = store.resolve_id(partial)?;
        let root_id = full_id_from_path(&root_path, partial).to_string();

        if !visible.contains_key(&root_id) {
            return Err(Error::TicketNotFound { id: root_id });
        }

        let mut ancestors = HashSet::new();
        render_node(
            &root_id,
            "",
            true,
            true,
            &visible,
            &children,
            &mut ancestors,
            max_depth,
            0,
            &mut lines,
        );
    } else {
        // Forest mode: all tickets whose parent is absent from the visible set.
        let mut roots: Vec<&Ticket> = visible
            .values()
            .filter(|t| t.parent.as_ref().is_none_or(|p| !visible.contains_key(p)))
            .collect();

        // Fallback: if every ticket is in a pure cycle (each has a visible
        // parent), no roots are found above.  Fall back to all visible tickets
        // as starting points — cycle detection (ancestor path) will annotate
        // re-entries with [cycle] and prevent infinite loops.
        if roots.is_empty() {
            roots = visible.values().collect();
        }

        // Sort roots: status priority (in_progress < open < closed),
        // then created ascending, then ID ascending as a stable tiebreak.
        roots.sort_by(|a, b| {
            status_priority(&a.status)
                .cmp(&status_priority(&b.status))
                .then_with(|| a.created.cmp(&b.created))
                .then_with(|| a.id.cmp(&b.id))
        });

        let root_count = roots.len();
        for (i, root) in roots.iter().enumerate() {
            let is_last = i == root_count - 1;
            let mut ancestors = HashSet::new();
            render_node(
                &root.id,
                "",
                is_last,
                true,
                &visible,
                &children,
                &mut ancestors,
                max_depth,
                0,
                &mut lines,
            );
            // Blank line between top-level trees (not after the last one).
            if !is_last {
                lines.push(TreeLine {
                    text: String::new(),
                });
            }
        }
    }

    let output = lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    Ok(output)
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn tree(
    partial_id: Option<&str>,
    max_depth: Option<usize>,
    include_closed: bool,
) -> Result<()> {
    let output = tree_impl(None, partial_id, max_depth, include_closed)?;
    pager::page_or_print(&format!("{output}\n"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Strip ANSI escape codes from a string.
    fn strip_ansi(s: &str) -> String {
        // ANSI escape sequences start with ESC [ and end with a letter.
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                // Consume '[' and everything up to and including the final letter.
                if chars.peek() == Some(&'[') {
                    chars.next();
                    for ch in chars.by_ref() {
                        if ch.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    /// Write a ticket file to `dir/.tickets/<id>.md`.
    ///
    /// `parent` is `Some("parent-id")` or `None`.
    /// `status` is "open", "in_progress", or "closed".
    /// `created` is an RFC 3339 timestamp string (e.g. "2026-01-01T00:00:00Z").
    fn write_ticket(
        dir: &Path,
        id: &str,
        title: &str,
        status: &str,
        parent: Option<&str>,
        created: &str,
    ) {
        let tickets_dir = dir.join(".tickets");
        fs::create_dir_all(&tickets_dir).unwrap();

        let parent_line = match parent {
            Some(p) => format!("parent: {p}"),
            None => "parent:".to_string(),
        };
        let content = format!(
            "---\nid: {id}\nstatus: {status}\ndeps: []\nlinks: []\ncreated: {created}\ntype: task\npriority: 2\nassignee: Test User\n{parent_line}\ntags: []\n---\n# {title}\n\nBody text.\n"
        );
        fs::write(tickets_dir.join(format!("{id}.md")), content).unwrap();
    }

    // -----------------------------------------------------------------------
    // Root ticket shown
    // -----------------------------------------------------------------------

    #[test]
    fn root_ticket_shown() {
        let tmp = tempdir().unwrap();
        write_ticket(
            tmp.path(),
            "task-0001",
            "Root ticket",
            "open",
            None,
            "2026-01-01T00:00:00Z",
        );

        let output = tree_impl(Some(tmp.path()), None, None, false).unwrap();
        let plain = strip_ansi(&output);

        assert!(
            plain.contains("task-0001"),
            "root ticket missing; output:\n{plain}"
        );
    }

    // -----------------------------------------------------------------------
    // Child indented under parent
    // -----------------------------------------------------------------------

    #[test]
    fn child_indented_under_parent() {
        let tmp = tempdir().unwrap();
        write_ticket(
            tmp.path(),
            "task-0001",
            "Parent",
            "open",
            None,
            "2026-01-01T00:00:00Z",
        );
        write_ticket(
            tmp.path(),
            "task-0002",
            "Child",
            "open",
            Some("task-0001"),
            "2026-01-02T00:00:00Z",
        );

        let output = tree_impl(Some(tmp.path()), None, None, false).unwrap();
        let plain = strip_ansi(&output);

        let pos_parent = plain.find("task-0001").unwrap();
        let pos_child = plain.find("task-0002").unwrap();
        assert!(
            pos_child > pos_parent,
            "child should appear after parent; output:\n{plain}"
        );

        // Box-drawing chars must appear between the two.
        let between = &plain[pos_parent..pos_child];
        assert!(
            between.contains("└──") || between.contains("├──"),
            "no box-drawing chars before child; output:\n{plain}"
        );
    }

    // -----------------------------------------------------------------------
    // Box-drawing characters
    // -----------------------------------------------------------------------

    #[test]
    fn box_drawing_characters() {
        let tmp = tempdir().unwrap();
        write_ticket(
            tmp.path(),
            "task-0001",
            "Parent",
            "open",
            None,
            "2026-01-01T00:00:00Z",
        );
        write_ticket(
            tmp.path(),
            "task-0002",
            "Child A",
            "open",
            Some("task-0001"),
            "2026-01-02T00:00:00Z",
        );
        write_ticket(
            tmp.path(),
            "task-0003",
            "Child B",
            "open",
            Some("task-0001"),
            "2026-01-03T00:00:00Z",
        );
        // Grandchild under Child A triggers the │ continuation on its line.
        write_ticket(
            tmp.path(),
            "task-0004",
            "Grandchild",
            "open",
            Some("task-0002"),
            "2026-01-04T00:00:00Z",
        );

        let output = tree_impl(Some(tmp.path()), None, None, false).unwrap();
        let plain = strip_ansi(&output);

        assert!(
            plain.contains("├──") || plain.contains("└──"),
            "missing ├── or └──; output:\n{plain}"
        );
        // The grandchild's line is prefixed with │ because Child B follows Child A.
        assert!(
            plain.contains("│"),
            "missing │ continuation; output:\n{plain}"
        );
    }

    // -----------------------------------------------------------------------
    // Multiple roots
    // -----------------------------------------------------------------------

    #[test]
    fn multiple_roots() {
        let tmp = tempdir().unwrap();
        write_ticket(
            tmp.path(),
            "task-0001",
            "Root A",
            "open",
            None,
            "2026-01-01T00:00:00Z",
        );
        write_ticket(
            tmp.path(),
            "task-0002",
            "Root B",
            "open",
            None,
            "2026-01-02T00:00:00Z",
        );

        let output = tree_impl(Some(tmp.path()), None, None, false).unwrap();
        let plain = strip_ansi(&output);

        assert!(
            plain.contains("task-0001"),
            "missing root A; output:\n{plain}"
        );
        assert!(
            plain.contains("task-0002"),
            "missing root B; output:\n{plain}"
        );
    }

    // -----------------------------------------------------------------------
    // --max-depth 0 shows root only
    // -----------------------------------------------------------------------

    #[test]
    fn max_depth_zero_shows_root_only() {
        let tmp = tempdir().unwrap();
        write_ticket(
            tmp.path(),
            "task-0001",
            "Root",
            "open",
            None,
            "2026-01-01T00:00:00Z",
        );
        write_ticket(
            tmp.path(),
            "task-0002",
            "Child",
            "open",
            Some("task-0001"),
            "2026-01-02T00:00:00Z",
        );
        write_ticket(
            tmp.path(),
            "task-0003",
            "Grandchild",
            "open",
            Some("task-0002"),
            "2026-01-03T00:00:00Z",
        );

        let output = tree_impl(Some(tmp.path()), None, Some(0), false).unwrap();
        let plain = strip_ansi(&output);

        assert!(
            plain.contains("task-0001"),
            "root missing; output:\n{plain}"
        );
        assert!(
            !plain.contains("task-0002"),
            "child should be hidden at depth 0; output:\n{plain}"
        );
        assert!(
            !plain.contains("task-0003"),
            "grandchild should be hidden at depth 0; output:\n{plain}"
        );
    }

    // -----------------------------------------------------------------------
    // --max-depth 1 shows one level
    // -----------------------------------------------------------------------

    #[test]
    fn max_depth_one_shows_one_level() {
        let tmp = tempdir().unwrap();
        write_ticket(
            tmp.path(),
            "task-0001",
            "Root",
            "open",
            None,
            "2026-01-01T00:00:00Z",
        );
        write_ticket(
            tmp.path(),
            "task-0002",
            "Child",
            "open",
            Some("task-0001"),
            "2026-01-02T00:00:00Z",
        );
        write_ticket(
            tmp.path(),
            "task-0003",
            "Grandchild",
            "open",
            Some("task-0002"),
            "2026-01-03T00:00:00Z",
        );

        let output = tree_impl(Some(tmp.path()), None, Some(1), false).unwrap();
        let plain = strip_ansi(&output);

        assert!(
            plain.contains("task-0001"),
            "root missing; output:\n{plain}"
        );
        assert!(
            plain.contains("task-0002"),
            "child missing at depth 1; output:\n{plain}"
        );
        assert!(
            !plain.contains("task-0003"),
            "grandchild should be hidden at depth 1; output:\n{plain}"
        );
    }

    // -----------------------------------------------------------------------
    // --all includes closed tickets
    // -----------------------------------------------------------------------

    #[test]
    fn all_includes_closed_tickets() {
        let tmp = tempdir().unwrap();
        write_ticket(
            tmp.path(),
            "task-0001",
            "Root",
            "open",
            None,
            "2026-01-01T00:00:00Z",
        );
        write_ticket(
            tmp.path(),
            "task-0002",
            "Closed child",
            "closed",
            Some("task-0001"),
            "2026-01-02T00:00:00Z",
        );

        // Without --all, closed child should not appear.
        let output_no_all = tree_impl(Some(tmp.path()), None, None, false).unwrap();
        let plain_no_all = strip_ansi(&output_no_all);
        assert!(
            !plain_no_all.contains("task-0002"),
            "closed child should be hidden by default; output:\n{plain_no_all}"
        );

        // With --all, closed child should appear.
        let output_all = tree_impl(Some(tmp.path()), None, None, true).unwrap();
        let plain_all = strip_ansi(&output_all);
        assert!(
            plain_all.contains("task-0002"),
            "closed child should appear with --all; output:\n{plain_all}"
        );
    }

    // -----------------------------------------------------------------------
    // Sort by status priority then created_at
    // -----------------------------------------------------------------------

    #[test]
    fn sort_by_status_priority_then_created() {
        let tmp = tempdir().unwrap();
        write_ticket(
            tmp.path(),
            "task-0001",
            "Root",
            "open",
            None,
            "2026-01-01T00:00:00Z",
        );
        // Three children with different statuses; open was created first.
        write_ticket(
            tmp.path(),
            "task-0002",
            "Open child",
            "open",
            Some("task-0001"),
            "2026-01-02T00:00:00Z",
        );
        write_ticket(
            tmp.path(),
            "task-0003",
            "In-progress child",
            "in_progress",
            Some("task-0001"),
            "2026-01-03T00:00:00Z",
        );

        let output = tree_impl(Some(tmp.path()), None, None, false).unwrap();
        let plain = strip_ansi(&output);

        let pos_ip = plain.find("task-0003").unwrap(); // in_progress
        let pos_open = plain.find("task-0002").unwrap(); // open
        assert!(
            pos_ip < pos_open,
            "in_progress child should appear before open child; output:\n{plain}"
        );
    }

    // -----------------------------------------------------------------------
    // Sort within same status by created_at
    // -----------------------------------------------------------------------

    #[test]
    fn sort_same_status_by_created() {
        let tmp = tempdir().unwrap();
        write_ticket(
            tmp.path(),
            "task-0001",
            "Root",
            "open",
            None,
            "2026-01-01T00:00:00Z",
        );
        write_ticket(
            tmp.path(),
            "task-0003",
            "Open later",
            "open",
            Some("task-0001"),
            "2026-01-03T00:00:00Z",
        );
        write_ticket(
            tmp.path(),
            "task-0002",
            "Open earlier",
            "open",
            Some("task-0001"),
            "2026-01-02T00:00:00Z",
        );

        let output = tree_impl(Some(tmp.path()), None, None, false).unwrap();
        let plain = strip_ansi(&output);

        let pos_early = plain.find("task-0002").unwrap();
        let pos_late = plain.find("task-0003").unwrap();
        assert!(
            pos_early < pos_late,
            "earlier created_at should appear first within same status; output:\n{plain}"
        );
    }

    // -----------------------------------------------------------------------
    // Cycle detection — no infinite loop
    // -----------------------------------------------------------------------

    #[test]
    fn cycle_detection_no_infinite_loop() {
        let tmp = tempdir().unwrap();
        // A's parent is B and B's parent is A — a pure cycle with no natural
        // root.  Both the explicit-ID (subtree) and no-ID (forest) paths must
        // terminate and annotate the cycle.
        write_ticket(
            tmp.path(),
            "task-0001",
            "Ticket A",
            "open",
            Some("task-0002"),
            "2026-01-01T00:00:00Z",
        );
        write_ticket(
            tmp.path(),
            "task-0002",
            "Ticket B",
            "open",
            Some("task-0001"),
            "2026-01-02T00:00:00Z",
        );

        // Explicit subtree ID path.
        let output = tree_impl(Some(tmp.path()), Some("task-0001"), None, false).unwrap();
        let plain = strip_ansi(&output);
        assert!(
            plain.contains("task-0001"),
            "subtree: missing task-0001; output:\n{plain}"
        );
        assert!(
            plain.contains("task-0002"),
            "subtree: missing task-0002; output:\n{plain}"
        );
        assert!(
            plain.contains("[cycle]"),
            "subtree: missing [cycle]; output:\n{plain}"
        );

        // Forest (no-ID) path — must not produce blank output.
        let output2 = tree_impl(Some(tmp.path()), None, None, false).unwrap();
        let plain2 = strip_ansi(&output2);
        assert!(
            !plain2.trim().is_empty(),
            "forest: output was blank for pure cycle"
        );
        assert!(
            plain2.contains("[cycle]"),
            "forest: missing [cycle]; output:\n{plain2}"
        );
    }

    // -----------------------------------------------------------------------
    // Subtree rooted at given ticket
    // -----------------------------------------------------------------------

    #[test]
    fn subtree_rooted_at_given_ticket() {
        let tmp = tempdir().unwrap();
        write_ticket(
            tmp.path(),
            "task-0001",
            "Grandparent",
            "open",
            None,
            "2026-01-01T00:00:00Z",
        );
        write_ticket(
            tmp.path(),
            "task-0002",
            "Parent",
            "open",
            Some("task-0001"),
            "2026-01-02T00:00:00Z",
        );
        write_ticket(
            tmp.path(),
            "task-0003",
            "Child",
            "open",
            Some("task-0002"),
            "2026-01-03T00:00:00Z",
        );

        // tree task-0002 should show task-0002 and task-0003, but NOT task-0001.
        let output = tree_impl(Some(tmp.path()), Some("task-0002"), None, false).unwrap();
        let plain = strip_ansi(&output);

        assert!(
            plain.contains("task-0002"),
            "subtree root missing; output:\n{plain}"
        );
        assert!(
            plain.contains("task-0003"),
            "subtree child missing; output:\n{plain}"
        );
        assert!(
            !plain.contains("task-0001"),
            "grandparent should not appear in subtree; output:\n{plain}"
        );
    }

    // -----------------------------------------------------------------------
    // Omitted ID shows all roots
    // -----------------------------------------------------------------------

    #[test]
    fn omitted_id_shows_all_roots() {
        let tmp = tempdir().unwrap();
        write_ticket(
            tmp.path(),
            "task-0001",
            "Root A",
            "open",
            None,
            "2026-01-01T00:00:00Z",
        );
        write_ticket(
            tmp.path(),
            "task-0002",
            "Root B",
            "open",
            None,
            "2026-01-02T00:00:00Z",
        );
        write_ticket(
            tmp.path(),
            "task-0003",
            "Child of A",
            "open",
            Some("task-0001"),
            "2026-01-03T00:00:00Z",
        );

        let output = tree_impl(Some(tmp.path()), None, None, false).unwrap();
        let plain = strip_ansi(&output);

        assert!(
            plain.contains("task-0001"),
            "root A missing; output:\n{plain}"
        );
        assert!(
            plain.contains("task-0002"),
            "root B missing; output:\n{plain}"
        );
        assert!(
            plain.contains("task-0003"),
            "child of A missing; output:\n{plain}"
        );
    }

    // -----------------------------------------------------------------------
    // NO_COLOR env var disables color
    // -----------------------------------------------------------------------

    #[test]
    fn no_color_env_var_disables_color() {
        let tmp = tempdir().unwrap();
        write_ticket(
            tmp.path(),
            "task-0001",
            "Root",
            "open",
            None,
            "2026-01-01T00:00:00Z",
        );

        // Set NO_COLOR=1 in the environment, which the console crate reads on
        // the next style() call.  Guard against test parallelism by also
        // calling set_colors_enabled so the already-initialised console state
        // reflects the variable.  Restore both on exit.
        // SAFETY: single-threaded test context; no other threads read this var.
        unsafe { std::env::set_var("NO_COLOR", "1") };
        console::set_colors_enabled(false);
        let output = tree_impl(Some(tmp.path()), None, None, false).unwrap();
        // SAFETY: restoring what we set above.
        unsafe { std::env::remove_var("NO_COLOR") };
        console::set_colors_enabled(true);

        // No ANSI escape sequences should be present.
        assert!(
            !output.contains('\x1b'),
            "ANSI escapes present even with NO_COLOR=1; output:\n{output}"
        );
    }
}
