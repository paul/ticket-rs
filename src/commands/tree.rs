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
// (in_progress < open < closed), then by dependency topological order within
// the same status group, then by ticket priority (P0 < P4) ascending, then
// by `created` ascending.
//
// Each line shows the ticket ID (dimmed), the colored priority label (P0–P4),
// the colored status label, the title, visible dependency IDs (colored by
// their status), and tags (dimmed, prefixed with #).  P0 = red, P1 =
// yellow/orange, P2 = magenta/purple, P3 = dim, P4 = black.  When
// stdout is a TTY, lines are truncated to the terminal width:
// tags are dropped first, then the title is truncated with "…" if the line
// is still too long, then deps are dropped as a last resort.  Truncation is
// disabled for piped output.
//
// Cycles in the parent chain are detected and annotated with `[cycle]` rather
// than causing infinite loops.
//
// Color respects the global `--color` flag and the `NO_COLOR` / `CLICOLOR`
// env vars via the `console` crate.  A blank line separates each top-level
// tree in forest mode.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use console::{Term, style};

use crate::error::{Error, Result};
use crate::format::{build_line, dep_id_label, priority_label, status_label};
use crate::pager;
use crate::store::TicketStore;
use crate::ticket::{Status, Ticket};

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Extract the ticket ID from a resolved `.md` path.
fn full_id_from_path<'a>(path: &'a Path, partial: &'a str) -> &'a str {
    path.file_stem().and_then(|s| s.to_str()).unwrap_or(partial)
}

/// Topologically sort `ids` (a slice of sibling ticket IDs) within a single
/// status group, respecting dependency edges where *both* endpoints are
/// present in the slice.  Tickets with no intra-group deps (or that are part
/// of a cycle) are ordered by `created` then `id` as a stable tiebreak.
///
/// This is Kahn's algorithm scoped to the sibling set.  Any nodes that remain
/// after the main pass (cycle members) are appended in `created`-then-`id`
/// order.
fn topo_sort_group(ids: &[String], tickets: &HashMap<String, Ticket>) -> Vec<String> {
    if ids.len() <= 1 {
        return ids.to_vec();
    }

    let id_set: HashSet<&str> = ids.iter().map(String::as_str).collect();

    // Build in-degree count and adjacency list (dep → dependents that need it).
    // An edge A → B means "B depends on A", so B must come after A.
    // In Kahn's terms: B has an incoming edge from A.
    let mut in_degree: HashMap<&str, usize> = ids.iter().map(|id| (id.as_str(), 0)).collect();
    // dependents[A] = list of Bs that depend on A (A must be emitted before B).
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

    for id in ids {
        let ticket = match tickets.get(id) {
            Some(t) => t,
            None => continue,
        };
        for dep in &ticket.deps {
            // Only consider edges where the dependency is also in this group.
            if id_set.contains(dep.as_str()) {
                // B (=id) depends on A (=dep): edge A → B.
                dependents
                    .entry(dep.as_str())
                    .or_default()
                    .push(id.as_str());
                *in_degree.entry(id.as_str()).or_insert(0) += 1;
            }
        }
    }

    // Stable initial ordering for the queue: priority then created then id.
    let mut sorted_ids = ids.to_vec();
    sorted_ids.sort_by(|a, b| {
        let ta = &tickets[a];
        let tb = &tickets[b];
        ta.priority
            .cmp(&tb.priority)
            .then_with(|| ta.created.cmp(&tb.created))
            .then_with(|| a.cmp(b))
    });

    // Kahn's BFS — seed with nodes that have no incoming edges (in-degree 0).
    let mut queue: VecDeque<&str> = VecDeque::new();
    for id in &sorted_ids {
        if in_degree[id.as_str()] == 0 {
            queue.push_back(id.as_str());
        }
    }

    let mut result: Vec<String> = Vec::with_capacity(ids.len());

    while let Some(id) = queue.pop_front() {
        result.push(id.to_string());
        if let Some(deps) = dependents.get(id) {
            // Maintain stable order when multiple nodes become available.
            let mut newly_ready: Vec<&str> = Vec::new();
            for &dep in deps {
                let deg = in_degree.get_mut(dep).expect("in_degree populated above");
                *deg -= 1;
                if *deg == 0 {
                    newly_ready.push(dep);
                }
            }
            // Sort newly ready nodes by priority then created then id before enqueueing.
            newly_ready.sort_by(|a, b| {
                let ta = &tickets[*a];
                let tb = &tickets[*b];
                ta.priority
                    .cmp(&tb.priority)
                    .then_with(|| ta.created.cmp(&tb.created))
                    .then_with(|| a.cmp(b))
            });
            for id in newly_ready {
                queue.push_back(id);
            }
        }
    }

    // Any remaining nodes are cycle members — append in created/id order.
    if result.len() < ids.len() {
        let emitted: HashSet<&str> = result.iter().map(String::as_str).collect();
        let mut remainder: Vec<&str> = ids
            .iter()
            .map(String::as_str)
            .filter(|id| !emitted.contains(*id))
            .collect();
        remainder.sort_by(|a, b| {
            let ta = &tickets[*a];
            let tb = &tickets[*b];
            ta.priority
                .cmp(&tb.priority)
                .then_with(|| ta.created.cmp(&tb.created))
                .then_with(|| a.cmp(b))
        });
        for id in remainder {
            result.push(id.to_string());
        }
    }

    result
}

/// Sort a child list: first by status priority, then by topological dependency
/// order within each status group, then by `created`, then by `id`.
fn sort_children(kids: &mut Vec<String>, tickets: &HashMap<String, Ticket>) {
    // Group by status priority.
    let mut groups: Vec<Vec<String>> = vec![Vec::new(); 3]; // 0=in_progress, 1=open, 2=closed
    for id in kids.drain(..) {
        let priority = tickets
            .get(&id)
            .map(|t| t.status.sort_key() as usize)
            .unwrap_or(1);
        groups[priority].push(id);
    }

    // Topologically sort each group, then reassemble.
    for group in &mut groups {
        let sorted = topo_sort_group(group, tickets);
        kids.extend(sorted);
    }
}

/// A single rendered line of tree output.
struct TreeLine {
    /// Content for the line (may contain ANSI color codes in the status badge).
    text: String,
}

/// Recursively render a node and all its visible descendants into `lines`.
///
/// - `id`             — ticket to render
/// - `prefix`         — accumulated indentation prefix for this node's children
/// - `is_last`        — whether this node is the last child of its parent
/// - `is_root`        — whether this node is a root (no connector drawn)
/// - `tickets`        — full id→ticket map (visible set only)
/// - `dep_statuses`   — id→status map for ALL tickets (used to render dep labels,
///                      including closed deps that are not in the visible set)
/// - `children`       — pre-built parent→children map (visible set only)
/// - `ancestors`      — set of IDs on the current DFS path (for cycle detection)
/// - `max_depth`      — optional depth limit (None = unlimited)
/// - `depth`          — current depth (root = 0)
/// - `term_width`     — optional terminal column count for line truncation
/// - `lines`          — output accumulator
#[allow(clippy::too_many_arguments)]
fn render_node(
    id: &str,
    prefix: &str,
    is_last: bool,
    is_root: bool,
    tickets: &HashMap<String, Ticket>,
    dep_statuses: &HashMap<String, Status>,
    children: &HashMap<String, Vec<String>>,
    ancestors: &mut HashSet<String>,
    max_depth: Option<usize>,
    depth: usize,
    term_width: Option<usize>,
    lines: &mut Vec<TreeLine>,
) {
    let ticket = tickets.get(id);
    let (status, priority, title, deps, tags) = match ticket {
        Some(t) => (
            t.status.clone(),
            t.priority,
            t.title.clone(),
            t.deps.clone(),
            t.tags.clone().unwrap_or_default(),
        ),
        None => (Status::Open, 2, "(not found)".to_string(), vec![], vec![]),
    };

    let (connector, child_prefix) = if is_root {
        (String::new(), String::new())
    } else if is_last {
        ("└── ".to_string(), format!("{prefix}    "))
    } else {
        ("├── ".to_string(), format!("{prefix}│   "))
    };

    // Cycle detection: if this ID is already on the ancestor path, emit the
    // full node text plus [cycle] and stop recursing (matches the original).
    if ancestors.contains(id) {
        let node = format!(
            "{} {} {} {} [cycle]",
            style(id).dim(),
            priority_label(priority),
            status_label(&status),
            title
        );
        lines.push(TreeLine {
            text: format!("{prefix}{connector}{node}"),
        });
        return;
    }

    // Build the fixed part of the line: prefix + connector + id + priority + status.
    let id_part = format!("{}", style(id).dim());
    let priority_part = priority_label(priority);
    let status_part = status_label(&status);

    // Build the deps suffix: show all known deps (open, in-progress, or closed).
    // Closed deps are rendered dim+strikethrough to show they are resolved.
    // Unknown dep IDs (not in dep_statuses at all) are omitted.
    let known_deps: Vec<&str> = deps
        .iter()
        .map(String::as_str)
        .filter(|dep_id| dep_statuses.contains_key(*dep_id))
        .collect();
    let deps_suffix = if known_deps.is_empty() {
        String::new()
    } else {
        let labeled: Vec<String> = known_deps
            .iter()
            .map(|dep_id| dep_id_label(dep_id, dep_statuses))
            .collect();
        format!(" [{}]", labeled.join(", "))
    };

    // Build the tags suffix.
    let tags_suffix = if tags.is_empty() {
        String::new()
    } else {
        let tag_strs: Vec<String> = tags.iter().map(|t| format!("#{t}")).collect();
        format!(" {}", style(tag_strs.join(" ")).dim())
    };

    // Build the full line and apply terminal-width truncation.
    let line_text = build_line(
        prefix,
        &connector,
        &id_part,
        &priority_part,
        &status_part,
        &title,
        &deps_suffix,
        &tags_suffix,
        term_width,
    );

    lines.push(TreeLine { text: line_text });

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
            dep_statuses,
            children,
            ancestors,
            max_depth,
            depth + 1,
            term_width,
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
///
/// `term_width` overrides the automatically detected terminal width.  `None`
/// means "detect from stdout if TTY, otherwise no truncation".  Tests pass
/// an explicit `Some(width)` or `Some(usize::MAX)` to control behaviour.
fn tree_impl(
    start_dir: Option<&Path>,
    partial_id: Option<&str>,
    max_depth: Option<usize>,
    include_closed: bool,
    term_width: Option<usize>,
) -> Result<String> {
    let store = TicketStore::find(start_dir)?;

    // Load all tickets.
    let all_tickets: Vec<Ticket> = store.list_tickets();

    if all_tickets.is_empty() {
        return Ok(format!(
            "-- Ticket Dir ({}) is empty --\n",
            store.dir().display()
        ));
    }

    // Build a status map for ALL tickets (including closed).  This is used by
    // dep rendering so that closed deps can be shown dim+strikethrough even
    // when they are not in the visible set.
    let dep_statuses: HashMap<String, Status> = all_tickets
        .iter()
        .map(|t| (t.id.clone(), t.status.clone()))
        .collect();

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

    // Sort each child list: status priority → topological dep order within
    // each status group → created → id.
    for kids in children.values_mut() {
        sort_children(kids, &visible);
    }

    let mut lines: Vec<TreeLine> = Vec::new();

    if let Some(partial) = partial_id {
        // Subtree mode: resolve the requested root.
        let root_path = store.resolve_id(partial)?;
        let root_id = full_id_from_path(&root_path, partial).to_string();

        if !visible.contains_key(&root_id) {
            return Err(Error::TicketNotFound {
                id: root_id,
                suggestions: vec![],
            });
        }

        let mut ancestors = HashSet::new();
        render_node(
            &root_id,
            "",
            true,
            true,
            &visible,
            &dep_statuses,
            &children,
            &mut ancestors,
            max_depth,
            0,
            term_width,
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
        // then ticket priority ascending, then created ascending, then ID
        // ascending as a stable tiebreak.
        roots.sort_by(|a, b| a.sort_cmp(b));

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
                &dep_statuses,
                &children,
                &mut ancestors,
                max_depth,
                0,
                term_width,
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
    // Detect terminal width when stdout is a TTY; disable truncation otherwise
    // (pipes, redirects) so scripted consumers see the full content.
    let term = Term::stdout();
    let term_width = if term.is_term() {
        let (_rows, cols) = term.size();
        Some(cols as usize)
    } else {
        None
    };

    let output = tree_impl(None, partial_id, max_depth, include_closed, term_width)?;
    pager::page_or_print(&format!("{output}\n"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::display_width;
    use std::fs;
    use tempfile::tempdir;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Strip ANSI escape codes from a string.
    fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
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
    /// `deps` is the list of dependency IDs.
    /// `tags` is the list of tags.
    ///
    /// Uses the default priority of 2.
    fn write_ticket(
        dir: &Path,
        id: &str,
        title: &str,
        status: &str,
        parent: Option<&str>,
        created: &str,
    ) {
        write_ticket_full(dir, id, title, status, parent, created, &[], &[], 2);
    }

    #[allow(clippy::too_many_arguments)]
    fn write_ticket_full(
        dir: &Path,
        id: &str,
        title: &str,
        status: &str,
        parent: Option<&str>,
        created: &str,
        deps: &[&str],
        tags: &[&str],
        priority: u8,
    ) {
        let tickets_dir = dir.join(".tickets");
        fs::create_dir_all(&tickets_dir).unwrap();

        let parent_line = match parent {
            Some(p) => format!("parent: {p}"),
            None => "parent:".to_string(),
        };
        let deps_str = deps.join(", ");
        let tags_str = tags
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let content = format!(
            "---\nid: {id}\nstatus: {status}\ndeps: [{deps_str}]\nlinks: []\ncreated: {created}\ntype: task\npriority: {priority}\nassignee: Test User\n{parent_line}\ntags: [{tags_str}]\n---\n# {title}\n\nBody text.\n"
        );
        fs::write(tickets_dir.join(format!("{id}.md")), content).unwrap();
    }

    // Shorthand: run tree_impl with no truncation (tests shouldn't truncate).
    fn run_tree(
        dir: &Path,
        partial_id: Option<&str>,
        max_depth: Option<usize>,
        include_closed: bool,
    ) -> String {
        tree_impl(Some(dir), partial_id, max_depth, include_closed, None).unwrap()
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

        let output = run_tree(tmp.path(), None, None, false);
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

        let output = run_tree(tmp.path(), None, None, false);
        let plain = strip_ansi(&output);

        let pos_parent = plain.find("task-0001").unwrap();
        let pos_child = plain.find("task-0002").unwrap();
        assert!(
            pos_child > pos_parent,
            "child should appear after parent; output:\n{plain}"
        );

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
        write_ticket(
            tmp.path(),
            "task-0004",
            "Grandchild",
            "open",
            Some("task-0002"),
            "2026-01-04T00:00:00Z",
        );

        let output = run_tree(tmp.path(), None, None, false);
        let plain = strip_ansi(&output);

        assert!(
            plain.contains("├──") || plain.contains("└──"),
            "missing ├── or └──; output:\n{plain}"
        );
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

        let output = run_tree(tmp.path(), None, None, false);
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

        let output = run_tree(tmp.path(), None, Some(0), false);
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

        let output = run_tree(tmp.path(), None, Some(1), false);
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

        let output_no_all = run_tree(tmp.path(), None, None, false);
        let plain_no_all = strip_ansi(&output_no_all);
        assert!(
            !plain_no_all.contains("task-0002"),
            "closed child should be hidden by default; output:\n{plain_no_all}"
        );

        let output_all = run_tree(tmp.path(), None, None, true);
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

        let output = run_tree(tmp.path(), None, None, false);
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

        let output = run_tree(tmp.path(), None, None, false);
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

        let output = tree_impl(Some(tmp.path()), Some("task-0001"), None, false, None).unwrap();
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

        let output2 = run_tree(tmp.path(), None, None, false);
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

        let output = tree_impl(Some(tmp.path()), Some("task-0002"), None, false, None).unwrap();
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

        let output = run_tree(tmp.path(), None, None, false);
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

        // SAFETY: single-threaded test context; no other threads read this var.
        unsafe { std::env::set_var("NO_COLOR", "1") };
        console::set_colors_enabled(false);
        let output = run_tree(tmp.path(), None, None, false);
        // SAFETY: restoring what we set above.
        unsafe { std::env::remove_var("NO_COLOR") };
        console::set_colors_enabled(true);

        assert!(
            !output.contains('\x1b'),
            "ANSI escapes present even with NO_COLOR=1; output:\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Dependency-aware sibling sorting
    // -----------------------------------------------------------------------

    /// Sibling B depends on sibling A — A must appear before B.
    #[test]
    fn dep_sort_b_after_a() {
        let tmp = tempdir().unwrap();
        write_ticket(
            tmp.path(),
            "task-0001",
            "Root",
            "open",
            None,
            "2026-01-01T00:00:00Z",
        );
        // task-0002 was created first but depends on task-0003.
        // task-0003 has no deps so it should appear first.
        write_ticket_full(
            tmp.path(),
            "task-0002",
            "Depends on 0003",
            "open",
            Some("task-0001"),
            "2026-01-02T00:00:00Z",
            &["task-0003"],
            &[],
            2,
        );
        write_ticket_full(
            tmp.path(),
            "task-0003",
            "No deps",
            "open",
            Some("task-0001"),
            "2026-01-03T00:00:00Z",
            &[],
            &[],
            2,
        );

        let output = run_tree(tmp.path(), None, None, false);
        let plain = strip_ansi(&output);

        let pos_0002 = plain.find("task-0002").unwrap();
        let pos_0003 = plain.find("task-0003").unwrap();
        assert!(
            pos_0003 < pos_0002,
            "task-0003 (no deps) should appear before task-0002 (depends on 0003); output:\n{plain}"
        );
    }

    /// Dep sort is confined to siblings — a cross-parent dep must not reorder
    /// sibling list of a different parent.
    #[test]
    fn dep_sort_cross_parent_does_not_affect_siblings() {
        let tmp = tempdir().unwrap();
        write_ticket(
            tmp.path(),
            "task-0001",
            "Root",
            "open",
            None,
            "2026-01-01T00:00:00Z",
        );
        // task-0002 depends on task-0010 which is a root (different parent).
        // This cross-parent dep should not change the order of task-0002 vs task-0003.
        write_ticket_full(
            tmp.path(),
            "task-0002",
            "Child A",
            "open",
            Some("task-0001"),
            "2026-01-02T00:00:00Z",
            &["task-0010"],
            &[],
            2,
        );
        write_ticket(
            tmp.path(),
            "task-0003",
            "Child B",
            "open",
            Some("task-0001"),
            "2026-01-03T00:00:00Z",
        );
        write_ticket(
            tmp.path(),
            "task-0010",
            "Other root",
            "open",
            None,
            "2026-01-01T00:00:00Z",
        );

        let output = run_tree(tmp.path(), None, None, false);
        let plain = strip_ansi(&output);

        // task-0002 and task-0003 both have no intra-sibling deps, so they
        // should remain in created order (task-0002 first).
        let pos_0002 = plain.find("task-0002").unwrap();
        let pos_0003 = plain.find("task-0003").unwrap();
        assert!(
            pos_0002 < pos_0003,
            "cross-parent dep should not reorder siblings; output:\n{plain}"
        );
    }

    /// Status priority takes precedence over dep ordering: an open ticket that
    /// depends on an in_progress sibling still appears after all in_progress
    /// siblings.
    #[test]
    fn dep_sort_status_overrides_deps() {
        let tmp = tempdir().unwrap();
        write_ticket(
            tmp.path(),
            "task-0001",
            "Root",
            "open",
            None,
            "2026-01-01T00:00:00Z",
        );
        // task-0002 is open and depends on task-0003 (in_progress).
        // Status priority puts in_progress before open regardless of dep order.
        write_ticket_full(
            tmp.path(),
            "task-0002",
            "Open depends on in_progress",
            "open",
            Some("task-0001"),
            "2026-01-02T00:00:00Z",
            &["task-0003"],
            &[],
            2,
        );
        write_ticket(
            tmp.path(),
            "task-0003",
            "In progress sibling",
            "in_progress",
            Some("task-0001"),
            "2026-01-03T00:00:00Z",
        );

        let output = run_tree(tmp.path(), None, None, false);
        let plain = strip_ansi(&output);

        let pos_ip = plain.find("task-0003").unwrap(); // in_progress
        let pos_open = plain.find("task-0002").unwrap(); // open
        assert!(
            pos_ip < pos_open,
            "in_progress sibling must appear before open sibling regardless of deps; output:\n{plain}"
        );
    }

    // -----------------------------------------------------------------------
    // Visible dependency IDs in output
    // -----------------------------------------------------------------------

    /// A visible dep ID appears in the output after the ticket's title.
    #[test]
    fn visible_dep_shown_in_output() {
        let tmp = tempdir().unwrap();
        write_ticket_full(
            tmp.path(),
            "task-0001",
            "Has dep",
            "open",
            None,
            "2026-01-01T00:00:00Z",
            &["task-0002"],
            &[],
            2,
        );
        write_ticket(
            tmp.path(),
            "task-0002",
            "The dep",
            "open",
            None,
            "2026-01-02T00:00:00Z",
        );

        let output = run_tree(tmp.path(), None, None, false);
        let plain = strip_ansi(&output);

        // The dep ID should appear after the ticket ID (which appears first).
        let pos_ticket = plain.find("task-0001").unwrap();
        // Find the second occurrence of task-0002 (first is its own line, second is in the deps list of task-0001).
        let line_with_dep = plain.lines().find(|l| l.contains("task-0001")).unwrap();
        assert!(
            line_with_dep.contains("task-0002"),
            "dep ID should appear on same line as ticket; line:\n{line_with_dep}"
        );
        let _ = pos_ticket; // used above
    }

    /// A dep that is NOT in the visible set (e.g. closed, not loaded) must not
    /// appear in the deps display.
    #[test]
    fn invisible_dep_not_shown() {
        let tmp = tempdir().unwrap();
        // task-0001 depends on task-0099 which does not exist in the store.
        write_ticket_full(
            tmp.path(),
            "task-0001",
            "Has invisible dep",
            "open",
            None,
            "2026-01-01T00:00:00Z",
            &["task-0099"],
            &[],
            2,
        );

        let output = run_tree(tmp.path(), None, None, false);
        let plain = strip_ansi(&output);

        assert!(
            !plain.contains("task-0099"),
            "invisible dep should not appear; output:\n{plain}"
        );
    }

    /// Closed deps are shown in the default view (include_closed=false) with
    /// dim+strikethrough styling to indicate they are resolved and not blocking.
    #[test]
    fn closed_dep_shown_with_strikethrough_by_default() {
        let tmp = tempdir().unwrap();
        write_ticket_full(
            tmp.path(),
            "task-0001",
            "Open ticket with closed dep",
            "open",
            None,
            "2026-01-01T00:00:00Z",
            &["task-0002"],
            &[],
            2,
        );
        write_ticket(
            tmp.path(),
            "task-0002",
            "Closed dep",
            "closed",
            None,
            "2026-01-02T00:00:00Z",
        );

        let output = run_tree(tmp.path(), None, None, false);
        let plain = strip_ansi(&output);

        // task-0002 is closed but should still appear in the dep list on
        // task-0001's line (dim+strikethrough) to show it is resolved.
        let line_0001 = plain.lines().find(|l| l.contains("task-0001")).unwrap();
        assert!(
            line_0001.contains("task-0002"),
            "closed dep should still appear on line (as resolved); line:\n{line_0001}"
        );

        // The closed dep itself should NOT appear as a tree node (it is not in
        // the visible set).
        let node_lines: Vec<&str> = plain
            .lines()
            .filter(|l| {
                l.starts_with("task-0002")
                    || l.contains("└──") && l.contains("task-0002")
                    || l.contains("├──") && l.contains("task-0002")
            })
            .collect();
        assert!(
            node_lines.is_empty(),
            "closed ticket should not appear as a tree node; found:\n{node_lines:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Tags in output
    // -----------------------------------------------------------------------

    /// Tags appear in the output when there is enough terminal width.
    #[test]
    fn tags_shown_when_width_permits() {
        let tmp = tempdir().unwrap();
        write_ticket_full(
            tmp.path(),
            "task-0001",
            "Tagged ticket",
            "open",
            None,
            "2026-01-01T00:00:00Z",
            &[],
            &["phase-1", "core"],
            2,
        );

        // Unlimited width — tags must appear.
        let output = run_tree(tmp.path(), None, None, false);
        let plain = strip_ansi(&output);

        assert!(
            plain.contains("#phase-1"),
            "#phase-1 tag missing; output:\n{plain}"
        );
        assert!(
            plain.contains("#core"),
            "#core tag missing; output:\n{plain}"
        );
    }

    /// Tags are omitted when the terminal is too narrow.
    #[test]
    fn tags_omitted_when_too_narrow() {
        let tmp = tempdir().unwrap();
        write_ticket_full(
            tmp.path(),
            "task-0001",
            "Tagged ticket",
            "open",
            None,
            "2026-01-01T00:00:00Z",
            &[],
            &["phase-1"],
            2,
        );

        // A very narrow terminal (30 cols) — the line is
        // "task-0001 open Tagged ticket #phase-1" which exceeds 30.
        let output = tree_impl(Some(tmp.path()), None, None, false, Some(30)).unwrap();
        let plain = strip_ansi(&output);

        assert!(
            !plain.contains("#phase-1"),
            "tag should be omitted when terminal is narrow; output:\n{plain}"
        );
        // The ticket itself should still be shown.
        assert!(
            plain.contains("task-0001"),
            "ticket missing; output:\n{plain}"
        );
    }

    // -----------------------------------------------------------------------
    // Terminal-width truncation
    // -----------------------------------------------------------------------

    /// A title that would overflow is truncated with "…".
    #[test]
    fn title_truncated_when_too_narrow() {
        let tmp = tempdir().unwrap();
        write_ticket(
            tmp.path(),
            "task-0001",
            "This is a very long title that should definitely be truncated",
            "open",
            None,
            "2026-01-01T00:00:00Z",
        );

        // Width of 30 cols should force truncation.
        let output = tree_impl(Some(tmp.path()), None, None, false, Some(30)).unwrap();
        let plain = strip_ansi(&output);

        assert!(
            plain.contains('…'),
            "truncated title should contain '…'; output:\n{plain}"
        );
        // The line must be at most 30 display columns.
        let line = plain.lines().next().unwrap();
        assert!(
            display_width(line) <= 30,
            "line exceeds 30 cols: width={}, line:\n{line}",
            display_width(line)
        );
    }

    /// When term_width is None (piped/non-TTY), no truncation happens.
    #[test]
    fn no_truncation_when_width_none() {
        let tmp = tempdir().unwrap();
        let long_title =
            "This is a very long title that should NOT be truncated when no width is set";
        write_ticket(
            tmp.path(),
            "task-0001",
            long_title,
            "open",
            None,
            "2026-01-01T00:00:00Z",
        );

        let output = run_tree(tmp.path(), None, None, false);
        let plain = strip_ansi(&output);

        assert!(
            plain.contains(long_title),
            "title should not be truncated when no term_width; output:\n{plain}"
        );
    }

    // -----------------------------------------------------------------------
    // Priority label in output
    // -----------------------------------------------------------------------

    /// Each ticket line shows its priority as a bracketed label (e.g. [P0]).
    #[test]
    fn priority_label_shown_in_output() {
        let tmp = tempdir().unwrap();
        write_ticket_full(
            tmp.path(),
            "task-0001",
            "Critical ticket",
            "open",
            None,
            "2026-01-01T00:00:00Z",
            &[],
            &[],
            0,
        );
        write_ticket_full(
            tmp.path(),
            "task-0002",
            "Low priority ticket",
            "open",
            None,
            "2026-01-02T00:00:00Z",
            &[],
            &[],
            4,
        );

        let output = run_tree(tmp.path(), None, None, false);
        let plain = strip_ansi(&output);

        let line_0001 = plain.lines().find(|l| l.contains("task-0001")).unwrap();
        assert!(
            line_0001.contains("P0"),
            "P0 label missing from line; line:\n{line_0001}"
        );

        let line_0002 = plain.lines().find(|l| l.contains("task-0002")).unwrap();
        assert!(
            line_0002.contains("P4"),
            "P4 label missing from line; line:\n{line_0002}"
        );
    }

    // -----------------------------------------------------------------------
    // Siblings sorted by priority within same status group
    // -----------------------------------------------------------------------

    /// Within the same status group, higher priority (lower number) siblings
    /// appear before lower priority (higher number) siblings.
    #[test]
    fn sort_siblings_by_priority_within_status_group() {
        let tmp = tempdir().unwrap();
        write_ticket(
            tmp.path(),
            "task-0001",
            "Root",
            "open",
            None,
            "2026-01-01T00:00:00Z",
        );
        // task-0002 is P4 created first; task-0003 is P0 created second.
        // P0 should appear before P4 despite the later created timestamp.
        write_ticket_full(
            tmp.path(),
            "task-0002",
            "Low priority child",
            "open",
            Some("task-0001"),
            "2026-01-02T00:00:00Z",
            &[],
            &[],
            4,
        );
        write_ticket_full(
            tmp.path(),
            "task-0003",
            "High priority child",
            "open",
            Some("task-0001"),
            "2026-01-03T00:00:00Z",
            &[],
            &[],
            0,
        );

        let output = run_tree(tmp.path(), None, None, false);
        let plain = strip_ansi(&output);

        let pos_p0 = plain.find("task-0003").unwrap(); // P0
        let pos_p4 = plain.find("task-0002").unwrap(); // P4
        assert!(
            pos_p0 < pos_p4,
            "P0 child should appear before P4 child; output:\n{plain}"
        );
    }

    /// When deps alone exceed the terminal width, the line must still fit within
    /// the budget (deps are dropped in the last-resort fallback).
    #[test]
    fn line_respects_width_when_deps_are_long() {
        let tmp = tempdir().unwrap();
        // task-0001 has many deps; their combined suffix is longer than the budget.
        write_ticket_full(
            tmp.path(),
            "task-0001",
            "Short",
            "open",
            None,
            "2026-01-01T00:00:00Z",
            &[
                "task-0002",
                "task-0003",
                "task-0004",
                "task-0005",
                "task-0006",
            ],
            &[],
            2,
        );
        // Create the dep tickets so they appear in the visible set.
        for (i, id) in [
            "task-0002",
            "task-0003",
            "task-0004",
            "task-0005",
            "task-0006",
        ]
        .iter()
        .enumerate()
        {
            write_ticket(
                tmp.path(),
                id,
                "Dep",
                "open",
                None,
                &format!("2026-01-0{}T00:00:00Z", i + 2),
            );
        }

        // Width of 30 cols — the deps suffix alone is ~55 chars.
        let output = tree_impl(Some(tmp.path()), None, None, false, Some(30)).unwrap();
        let plain = strip_ansi(&output);

        // Every line must fit within the budget.
        for line in plain.lines() {
            assert!(
                display_width(line) <= 30,
                "line exceeds 30 cols (width={}): {line}",
                display_width(line)
            );
        }
        // task-0001 must still appear.
        assert!(
            plain.contains("task-0001"),
            "ticket missing; output:\n{plain}"
        );
    }
}
