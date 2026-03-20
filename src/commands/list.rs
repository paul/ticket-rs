// Implementation of the `ls` / `list`, `ready`, `blocked`, and `closed`
// subcommands.
//
// All list output uses the shared `build_line` formatter from `crate::format`
// so that every command produces the same ticket-line style:
//
//   {id} {priority} {status} {title}[ [{deps}]][ {#tags}]
//
// Colors and terminal-width truncation are applied when stdout is a TTY;
// truncation is disabled for piped output so scripted consumers get the full
// content.

use std::collections::HashMap;
use std::path::Path;

use console::{Term, style};

use crate::error::Result;
use crate::format::{build_line, dep_id_label, priority_label, status_label};
use crate::pager;
use crate::store::TicketStore;
use crate::ticket::{Status, Ticket};

// ---------------------------------------------------------------------------
// Empty-dir helper
// ---------------------------------------------------------------------------

/// Return the message shown when a ticket directory exists but contains no
/// tickets matching the current command and filters.
pub(crate) fn empty_dir_message(dir: &Path) -> String {
    format!("-- Ticket Dir ({}) is empty --\n", dir.display())
}

// ---------------------------------------------------------------------------
// TTY detection helper
// ---------------------------------------------------------------------------

pub(crate) fn tty_width() -> Option<usize> {
    let term = Term::stdout();
    if term.is_term() {
        let (_rows, cols) = term.size();
        Some(cols as usize)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Shared single-ticket line formatter
// ---------------------------------------------------------------------------

/// Format a single ticket as a display line using the canonical format.
///
/// `by_id` is used to color dep IDs by their status.  Pass an empty map when
/// dep coloring is not needed (e.g. `ready`, which has no dep suffix).
/// `deps_override` replaces the ticket's own deps list (used by `blocked` to
/// show only the unclosed subset).  Pass `None` to use `ticket.deps`.
pub(crate) fn ticket_line(
    ticket: &Ticket,
    dep_statuses: &HashMap<String, Status>,
    deps_override: Option<&[String]>,
    term_width: Option<usize>,
) -> String {
    let id_part = format!("{}", style(&ticket.id).dim());
    let priority_part = priority_label(ticket.priority);
    let status_part = status_label(&ticket.status);

    let dep_ids = deps_override.unwrap_or(&ticket.deps);
    let deps_suffix = if dep_ids.is_empty() {
        String::new()
    } else {
        let labeled: Vec<String> = dep_ids
            .iter()
            .map(|dep_id| dep_id_label(dep_id, dep_statuses))
            .collect();
        format!(" [{}]", labeled.join(", "))
    };

    let tags_suffix = match &ticket.tags {
        Some(tags) if !tags.is_empty() => {
            let tag_strs: Vec<String> = tags.iter().map(|t| format!("#{t}")).collect();
            format!(" {}", style(tag_strs.join(" ")).dim())
        }
        _ => String::new(),
    };

    build_line(
        "",
        "",
        &id_part,
        &priority_part,
        &status_part,
        &ticket.title,
        &deps_suffix,
        &tags_suffix,
        term_width,
    )
}

// ---------------------------------------------------------------------------
// ls / list
// ---------------------------------------------------------------------------

/// List tickets, optionally filtered by status, assignee, and/or tag.
pub fn ls(status: Option<&str>, assignee: Option<&str>, tag: Option<&str>) -> Result<()> {
    let output = ls_impl(None, status, assignee, tag, tty_width())?;
    pager::page_or_print(&output)
}

/// Core logic for `ls`.  `start_dir` is passed to `TicketStore::find`; `None`
/// uses the cwd.  Returns the full formatted output string, or an empty-dir
/// message when the ticket directory exists but contains no tickets.
fn ls_impl(
    start_dir: Option<&Path>,
    status: Option<&str>,
    assignee: Option<&str>,
    tag: Option<&str>,
    term_width: Option<usize>,
) -> Result<String> {
    let store = TicketStore::find(start_dir)?;
    let tickets = store.list_tickets();
    if tickets.is_empty() {
        return Ok(empty_dir_message(store.dir()));
    }
    format_list(&tickets, status, assignee, tag, term_width)
}

/// Apply filters, sort, and format a slice of tickets into the output string.
///
/// Extracted so that unit tests can call it directly with in-memory ticket
/// slices without touching the filesystem.
pub(crate) fn format_list(
    tickets: &[Ticket],
    status: Option<&str>,
    assignee: Option<&str>,
    tag: Option<&str>,
    term_width: Option<usize>,
) -> Result<String> {
    // Parse the status filter once up-front so we can return an error early.
    let status_filter: Option<Status> = status.map(|s| s.parse::<Status>()).transpose()?;

    // Build a status lookup map for dep coloring.
    let dep_statuses: HashMap<String, Status> = tickets
        .iter()
        .map(|t| (t.id.clone(), t.status.clone()))
        .collect();

    // Apply filters.
    let mut filtered: Vec<&Ticket> = tickets
        .iter()
        .filter(|t| {
            if let Some(ref s) = status_filter
                && &t.status != s
            {
                return false;
            }
            t.matches_filters(assignee, tag)
        })
        .collect();

    // Sort: status priority, then ticket priority, then created, then ID.
    filtered.sort_by(|a, b| a.sort_cmp(b));

    let lines: Vec<String> = filtered
        .iter()
        .map(|t| ticket_line(t, &dep_statuses, None, term_width))
        .collect();

    if lines.is_empty() {
        return Ok(String::new());
    }

    Ok(lines.join("\n") + "\n")
}

// ---------------------------------------------------------------------------
// ready
// ---------------------------------------------------------------------------

/// Show tickets that are ready to work on (open or in-progress, all deps closed).
pub fn ready(assignee: Option<&str>, tag: Option<&str>) -> Result<()> {
    let output = ready_impl(None, assignee, tag, tty_width())?;
    pager::page_or_print(&output)
}

/// Core logic for `ready`.
fn ready_impl(
    start_dir: Option<&Path>,
    assignee: Option<&str>,
    tag: Option<&str>,
    term_width: Option<usize>,
) -> Result<String> {
    let store = TicketStore::find(start_dir)?;
    let tickets = store.list_tickets();
    if tickets.is_empty() {
        return Ok(empty_dir_message(store.dir()));
    }
    Ok(format_ready(&tickets, assignee, tag, term_width))
}

/// Filter to ready tickets, sort, and format into the output string.
///
/// A ticket is "ready" when:
///   - Its status is `open` or `in_progress` (not closed).
///   - Every ID in its `deps` list resolves to a ticket whose status is `closed`.
///     If a dep ID is not found in the store at all, the ticket is treated as blocked.
///
/// Extracted so that unit tests can call it directly with in-memory ticket
/// slices without touching the filesystem.
pub(crate) fn format_ready(
    tickets: &[Ticket],
    assignee: Option<&str>,
    tag: Option<&str>,
    term_width: Option<usize>,
) -> String {
    // Build a status lookup map so dependency checks are O(1).
    let dep_statuses: HashMap<String, Status> = tickets
        .iter()
        .map(|t| (t.id.clone(), t.status.clone()))
        .collect();

    let mut filtered: Vec<&Ticket> = tickets
        .iter()
        .filter(|t| {
            // Exclude closed tickets.
            if t.status == Status::Closed {
                return false;
            }
            // Exclude tickets with any unclosed (or unresolvable) dependency.
            if !t
                .deps
                .iter()
                .all(|dep_id| matches!(dep_statuses.get(dep_id.as_str()), Some(Status::Closed)))
            {
                return false;
            }
            // Optional assignee and tag filters.
            t.matches_filters(assignee, tag)
        })
        .collect();

    // Sort: status priority, then ticket priority, then created, then ID.
    filtered.sort_by(|a, b| a.sort_cmp(b));

    // Ready tickets have no relevant dep suffix (all deps are closed).
    let empty: HashMap<String, Status> = HashMap::new();
    let lines: Vec<String> = filtered
        .iter()
        .map(|t| ticket_line(t, &empty, Some(&[]), term_width))
        .collect();

    if lines.is_empty() {
        return String::new();
    }

    lines.join("\n") + "\n"
}

// ---------------------------------------------------------------------------
// blocked
// ---------------------------------------------------------------------------

/// Show tickets that are blocked by at least one unclosed dependency.
pub fn blocked(assignee: Option<&str>, tag: Option<&str>) -> Result<()> {
    let output = blocked_impl(None, assignee, tag, tty_width())?;
    pager::page_or_print(&output)
}

/// Core logic for `blocked`.
fn blocked_impl(
    start_dir: Option<&Path>,
    assignee: Option<&str>,
    tag: Option<&str>,
    term_width: Option<usize>,
) -> Result<String> {
    let store = TicketStore::find(start_dir)?;
    let tickets = store.list_tickets();
    if tickets.is_empty() {
        return Ok(empty_dir_message(store.dir()));
    }
    Ok(format_blocked(&tickets, assignee, tag, term_width))
}

/// Filter to blocked tickets, sort, and format into the output string.
///
/// A ticket is "blocked" when:
///   - Its status is `open` or `in_progress` (not closed).
///   - At least one ID in its `deps` list resolves to a ticket whose status is
///     NOT `closed`, or the dep ID cannot be found in the store at all.
///
/// Extracted so that unit tests can call it directly with in-memory ticket
/// slices without touching the filesystem.
pub(crate) fn format_blocked(
    tickets: &[Ticket],
    assignee: Option<&str>,
    tag: Option<&str>,
    term_width: Option<usize>,
) -> String {
    // Build a status lookup map so dependency checks and dep coloring are O(1).
    let dep_statuses: HashMap<String, Status> = tickets
        .iter()
        .map(|t| (t.id.clone(), t.status.clone()))
        .collect();

    let mut filtered: Vec<&Ticket> = tickets
        .iter()
        .filter(|t| {
            // Exclude closed tickets.
            if t.status == Status::Closed {
                return false;
            }
            // Must have at least one unclosed (or unresolvable) dependency.
            if t.deps.is_empty() {
                return false;
            }
            let has_unclosed_dep = t
                .deps
                .iter()
                .any(|dep_id| !matches!(dep_statuses.get(dep_id.as_str()), Some(Status::Closed)));
            if !has_unclosed_dep {
                return false;
            }
            // Optional assignee and tag filters.
            t.matches_filters(assignee, tag)
        })
        .collect();

    // Sort: status priority, then ticket priority, then created, then ID.
    filtered.sort_by(|a, b| a.sort_cmp(b));

    let lines: Vec<String> = filtered
        .iter()
        .map(|t| {
            // Only show the unclosed (or unresolvable) deps in the suffix.
            let open_deps: Vec<String> = t
                .deps
                .iter()
                .filter(|dep_id| !matches!(dep_statuses.get(dep_id.as_str()), Some(Status::Closed)))
                .cloned()
                .collect();
            ticket_line(t, &dep_statuses, Some(&open_deps), term_width)
        })
        .collect();

    if lines.is_empty() {
        return String::new();
    }

    lines.join("\n") + "\n"
}

// ---------------------------------------------------------------------------
// closed
// ---------------------------------------------------------------------------

/// Show recently closed tickets sorted by file modification time (most recent first).
pub fn closed(limit: usize, assignee: Option<&str>, tag: Option<&str>) -> Result<()> {
    let output = closed_impl(None, limit, assignee, tag, tty_width())?;
    pager::page_or_print(&output)
}

/// Core logic for `closed`.
fn closed_impl(
    start_dir: Option<&Path>,
    limit: usize,
    assignee: Option<&str>,
    tag: Option<&str>,
    term_width: Option<usize>,
) -> Result<String> {
    let store = TicketStore::find(start_dir)?;
    // Retrieve paths sorted by mtime (most recent first) and read only
    // as many as needed for efficiency.
    let paths = store.paths_by_mtime();
    if paths.is_empty() {
        return Ok(empty_dir_message(store.dir()));
    }
    Ok(format_closed_from_paths(
        &paths, limit, assignee, tag, term_width,
    ))
}

/// Filter closed tickets from a mtime-sorted path list, apply filters, and
/// format the output.
///
/// Only inspects the first `limit` files (by mtime) for efficiency, then
/// filters by status, assignee, and tag within that bounded candidate set.
///
/// Extracted so unit tests can call it directly.
pub(crate) fn format_closed_from_paths(
    paths: &[std::path::PathBuf],
    limit: usize,
    assignee: Option<&str>,
    tag: Option<&str>,
    term_width: Option<usize>,
) -> String {
    // Closed tickets have no relevant dep suffix.
    let empty: HashMap<String, Status> = HashMap::new();
    let results: Vec<String> = paths
        .iter()
        .take(limit)
        .filter_map(|path| {
            let content = std::fs::read_to_string(path).ok()?;
            let ticket = crate::ticket::Ticket::read_from_str(&content).ok()?;
            if ticket.status != crate::ticket::Status::Closed {
                return None;
            }
            // Optional assignee and tag filters.
            if !ticket.matches_filters(assignee, tag) {
                return None;
            }
            Some(ticket_line(&ticket, &empty, Some(&[]), term_width))
        })
        .collect();

    if results.is_empty() {
        return String::new();
    }

    results.join("\n") + "\n"
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    use crate::ticket::{Status, Ticket, TicketType};

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Build a minimal `Ticket` for testing without filesystem access.
    fn make_ticket(id: &str, title: &str) -> Ticket {
        Ticket {
            id: id.to_string(),
            status: Status::Open,
            deps: vec![],
            links: vec![],
            created: Utc::now(),
            ticket_type: TicketType::Task,
            priority: 2,
            assignee: None,
            external_ref: None,
            parent: None,
            tags: None,
            title: title.to_string(),
            body: format!("# {title}\n"),
        }
    }

    fn with_status(mut t: Ticket, s: Status) -> Ticket {
        t.status = s;
        t
    }

    fn with_assignee(mut t: Ticket, a: &str) -> Ticket {
        t.assignee = Some(a.to_string());
        t
    }

    fn with_tags(mut t: Ticket, tags: &[&str]) -> Ticket {
        t.tags = Some(tags.iter().map(|s| s.to_string()).collect());
        t
    }

    fn with_deps(mut t: Ticket, deps: &[&str]) -> Ticket {
        t.deps = deps.iter().map(|s| s.to_string()).collect();
        t
    }

    fn with_priority(mut t: Ticket, p: u8) -> Ticket {
        t.priority = p;
        t
    }

    fn with_created(mut t: Ticket, ts: chrono::DateTime<Utc>) -> Ticket {
        t.created = ts;
        t
    }

    // -----------------------------------------------------------------------
    // Lists all tickets
    // -----------------------------------------------------------------------

    #[test]
    fn lists_all_tickets() {
        let tickets = vec![
            make_ticket("tr-0001", "First"),
            make_ticket("tr-0002", "Second"),
        ];
        let output = format_list(&tickets, None, None, None, None).unwrap();
        assert!(
            output.contains("tr-0001"),
            "expected tr-0001 in output\n{output}"
        );
        assert!(
            output.contains("tr-0002"),
            "expected tr-0002 in output\n{output}"
        );
        assert!(
            output.contains("First"),
            "expected title 'First' in output\n{output}"
        );
        assert!(
            output.contains("Second"),
            "expected title 'Second' in output\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Output format
    // -----------------------------------------------------------------------

    #[test]
    fn output_format_matches_expected() {
        let tickets = vec![make_ticket("list-0001", "My ticket")];
        let output = format_list(&tickets, None, None, None, None).unwrap();
        // Format: {id} {priority} {status} {title}
        let line = output.trim_end_matches('\n');
        assert!(line.contains("open"), "expected 'open' in line\n{line}");
        assert!(
            line.contains("My ticket"),
            "expected 'My ticket' in line\n{line}"
        );
        assert!(
            line.starts_with("list-0001"),
            "expected line to start with 'list-0001'\n{line}"
        );
    }

    // -----------------------------------------------------------------------
    // Deps shown when non-empty
    // -----------------------------------------------------------------------

    #[test]
    fn deps_shown_when_non_empty() {
        let tickets = vec![with_deps(make_ticket("tr-0001", "Main"), &["dep-001"])];
        let output = format_list(&tickets, None, None, None, None).unwrap();
        assert!(
            output.contains("[dep-001]"),
            "expected '[dep-001]' in output\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Deps hidden when empty
    // -----------------------------------------------------------------------

    #[test]
    fn deps_hidden_when_empty() {
        let tickets = vec![make_ticket("tr-0001", "No deps")];
        let output = format_list(&tickets, None, None, None, None).unwrap();
        assert!(
            !output.contains("<-"),
            "expected no '<-' in output when deps is empty\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // --status filter: keeps matching
    // -----------------------------------------------------------------------

    #[test]
    fn status_filter_keeps_matching() {
        let tickets = vec![
            make_ticket("tr-0001", "Open ticket"),
            with_status(make_ticket("tr-0002", "Closed ticket"), Status::Closed),
        ];
        let output = format_list(&tickets, Some("open"), None, None, None).unwrap();
        assert!(
            output.contains("tr-0001"),
            "expected open ticket in output\n{output}"
        );
        assert!(
            !output.contains("tr-0002"),
            "expected closed ticket excluded\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // --status filter: excludes non-matching
    // -----------------------------------------------------------------------

    #[test]
    fn status_filter_excludes_non_matching() {
        let tickets = vec![
            with_status(make_ticket("tr-0001", "Closed"), Status::Closed),
            with_status(make_ticket("tr-0002", "Also closed"), Status::Closed),
        ];
        let output = format_list(&tickets, Some("open"), None, None, None).unwrap();
        assert!(
            output.is_empty(),
            "expected empty output when no open tickets\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // -a/--assignee filter
    // -----------------------------------------------------------------------

    #[test]
    fn assignee_filter() {
        let tickets = vec![
            with_assignee(make_ticket("tr-0001", "Alice ticket"), "Alice"),
            with_assignee(make_ticket("tr-0002", "Bob ticket"), "Bob"),
        ];
        let output = format_list(&tickets, None, Some("Alice"), None, None).unwrap();
        assert!(
            output.contains("tr-0001"),
            "expected Alice's ticket\n{output}"
        );
        assert!(
            !output.contains("tr-0002"),
            "expected Bob's ticket excluded\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // -T/--tag filter
    // -----------------------------------------------------------------------

    #[test]
    fn tag_filter() {
        let tickets = vec![
            with_tags(
                make_ticket("tr-0001", "Backend ticket"),
                &["backend", "api"],
            ),
            with_tags(make_ticket("tr-0002", "Frontend ticket"), &["frontend"]),
        ];
        let output = format_list(&tickets, None, None, Some("backend"), None).unwrap();
        assert!(
            output.contains("tr-0001"),
            "expected backend ticket\n{output}"
        );
        assert!(
            !output.contains("tr-0002"),
            "expected frontend ticket excluded\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Sort by priority then ID
    // -----------------------------------------------------------------------

    #[test]
    fn sort_by_priority_then_id() {
        // Pin created to the same timestamp so ID is the stable tiebreaker.
        let t0 = chrono::DateTime::from_timestamp(0, 0).unwrap();
        let tickets = vec![
            with_created(with_priority(make_ticket("c", "Low priority"), 3), t0),
            with_created(with_priority(make_ticket("b", "High priority B"), 1), t0),
            with_created(with_priority(make_ticket("a", "High priority A"), 1), t0),
        ];
        let output = format_list(&tickets, None, None, None, None).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3, "expected 3 lines\n{output}");
        assert!(
            lines[0].starts_with("a"),
            "expected 'a' first (priority 1, id a)\n{output}"
        );
        assert!(
            lines[1].starts_with("b"),
            "expected 'b' second (priority 1, id b)\n{output}"
        );
        assert!(
            lines[2].starts_with("c"),
            "expected 'c' third (priority 3)\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Empty list produces empty output
    // -----------------------------------------------------------------------

    #[test]
    fn empty_list_produces_empty_output() {
        let tickets: Vec<Ticket> = vec![];
        let output = format_list(&tickets, None, None, None, None).unwrap();
        assert!(
            output.is_empty(),
            "expected empty output for empty ticket list"
        );
    }

    // -----------------------------------------------------------------------
    // Empty dir message — ls/ready/blocked/closed on empty store
    // -----------------------------------------------------------------------

    #[test]
    fn ls_impl_empty_store_shows_empty_dir_message() {
        let root = tempfile::tempdir().unwrap();
        let tickets_dir = root.path().join(".tickets");
        std::fs::create_dir_all(&tickets_dir).unwrap();

        let output = ls_impl(Some(root.path()), None, None, None, None).unwrap();
        assert!(
            output.contains("-- Ticket Dir ("),
            "expected empty-dir message, got: {output}"
        );
        assert!(
            output.contains("is empty --"),
            "expected empty-dir message, got: {output}"
        );
        assert!(
            output.contains(tickets_dir.to_str().unwrap()),
            "expected dir path in message, got: {output}"
        );
    }

    #[test]
    fn ready_impl_empty_store_shows_empty_dir_message() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join(".tickets")).unwrap();

        let output = ready_impl(Some(root.path()), None, None, None).unwrap();
        assert!(
            output.contains("is empty --"),
            "expected empty-dir message, got: {output}"
        );
    }

    #[test]
    fn blocked_impl_empty_store_shows_empty_dir_message() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join(".tickets")).unwrap();

        let output = blocked_impl(Some(root.path()), None, None, None).unwrap();
        assert!(
            output.contains("is empty --"),
            "expected empty-dir message, got: {output}"
        );
    }

    #[test]
    fn closed_impl_empty_store_shows_empty_dir_message() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join(".tickets")).unwrap();

        let output = closed_impl(Some(root.path()), 20, None, None, None).unwrap();
        assert!(
            output.contains("is empty --"),
            "expected empty-dir message, got: {output}"
        );
    }

    // -----------------------------------------------------------------------
    // `list` alias: the CLI parses `list` as the Ls variant
    // -----------------------------------------------------------------------

    #[test]
    fn list_alias_parses_to_ls_variant() {
        use crate::cli::{Cli, Commands};
        use clap::Parser;

        // Both `ls` and `list` should parse to the Ls variant.
        for sub in ["ls", "list"] {
            let cli = Cli::try_parse_from(["ticket", sub])
                .unwrap_or_else(|e| panic!("failed to parse 'ticket {sub}': {e}"));
            assert!(
                matches!(cli.command, Commands::Ls { .. }),
                "expected Ls variant for subcommand '{sub}'"
            );
        }
    }

    // =======================================================================
    // ready command tests
    // =======================================================================

    // -----------------------------------------------------------------------
    // Ticket with no deps is ready
    // -----------------------------------------------------------------------

    #[test]
    fn ready_ticket_with_no_deps_is_ready() {
        let tickets = vec![make_ticket("tr-0001", "No deps ticket")];
        let output = format_ready(&tickets, None, None, None);
        assert!(
            output.contains("tr-0001"),
            "expected ticket with no deps to appear in ready output\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Ticket with all deps closed is ready
    // -----------------------------------------------------------------------

    #[test]
    fn ready_ticket_with_all_deps_closed_is_ready() {
        let tickets = vec![
            with_deps(make_ticket("tr-0001", "Main ticket"), &["tr-dep"]),
            with_status(make_ticket("tr-dep", "Closed dep"), Status::Closed),
        ];
        let output = format_ready(&tickets, None, None, None);
        assert!(
            output.contains("tr-0001"),
            "expected ticket with all closed deps to appear in ready output\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Ticket with any unclosed dep is not ready
    // -----------------------------------------------------------------------

    #[test]
    fn ready_ticket_with_unclosed_dep_is_not_ready() {
        let tickets = vec![
            with_deps(make_ticket("tr-0001", "Blocked ticket"), &["tr-dep"]),
            make_ticket("tr-dep", "Open dep"), // status defaults to open
        ];
        let output = format_ready(&tickets, None, None, None);
        assert!(
            !output.contains("tr-0001"),
            "expected ticket with open dep to be excluded from ready output\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Closed ticket is not ready
    // -----------------------------------------------------------------------

    #[test]
    fn ready_closed_ticket_is_not_ready() {
        let tickets = vec![with_status(
            make_ticket("tr-0001", "Closed ticket"),
            Status::Closed,
        )];
        let output = format_ready(&tickets, None, None, None);
        assert!(
            !output.contains("tr-0001"),
            "expected closed ticket to be excluded from ready output\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // in_progress ticket with all deps closed is ready
    // -----------------------------------------------------------------------

    #[test]
    fn ready_in_progress_ticket_with_closed_deps_is_ready() {
        let tickets = vec![
            with_status(
                with_deps(make_ticket("tr-0001", "In progress ticket"), &["tr-dep"]),
                Status::InProgress,
            ),
            with_status(make_ticket("tr-dep", "Closed dep"), Status::Closed),
        ];
        let output = format_ready(&tickets, None, None, None);
        assert!(
            output.contains("tr-0001"),
            "expected in_progress ticket with closed deps to appear in ready output\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Priority badge format
    // -----------------------------------------------------------------------

    #[test]
    fn ready_priority_badge_format() {
        let tickets = vec![make_ticket("ready-001", "Priority ticket")]; // priority defaults to 2
        let output = format_ready(&tickets, None, None, None);
        let line = output.trim_end_matches('\n');
        // Format: {id} {priority} {status} {title}
        assert!(
            line.starts_with("ready-001"),
            "expected id at start\n{line}"
        );
        assert!(line.contains("P2"), "expected priority 'P2'\n{line}");
        assert!(line.contains("open"), "expected status 'open'\n{line}");
        assert!(
            line.contains("Priority ticket"),
            "expected title in line\n{line}"
        );
    }

    // -----------------------------------------------------------------------
    // Sort by priority then ID
    // -----------------------------------------------------------------------

    #[test]
    fn ready_sort_by_priority_then_id() {
        // Pin created to the same timestamp so ID is the stable tiebreaker.
        let t0 = chrono::DateTime::from_timestamp(0, 0).unwrap();
        let tickets = vec![
            with_created(with_priority(make_ticket("c", "Low priority"), 3), t0),
            with_created(with_priority(make_ticket("b", "Also high priority"), 1), t0),
            with_created(with_priority(make_ticket("a", "High priority"), 1), t0),
        ];
        let output = format_ready(&tickets, None, None, None);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3, "expected 3 lines\n{output}");
        assert!(
            lines[0].starts_with("a"),
            "expected 'a' first (priority 1, id a)\n{output}"
        );
        assert!(
            lines[1].starts_with("b"),
            "expected 'b' second (priority 1, id b)\n{output}"
        );
        assert!(
            lines[2].starts_with("c"),
            "expected 'c' third (priority 3)\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // -a/--assignee filter
    // -----------------------------------------------------------------------

    #[test]
    fn ready_assignee_filter() {
        let tickets = vec![
            with_assignee(make_ticket("tr-0001", "Alice ticket"), "Alice"),
            with_assignee(make_ticket("tr-0002", "Bob ticket"), "Bob"),
        ];
        let output = format_ready(&tickets, Some("Alice"), None, None);
        assert!(
            output.contains("tr-0001"),
            "expected Alice's ticket in ready output\n{output}"
        );
        assert!(
            !output.contains("tr-0002"),
            "expected Bob's ticket excluded from ready output\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // -T/--tag filter
    // -----------------------------------------------------------------------

    #[test]
    fn ready_tag_filter() {
        let tickets = vec![
            with_tags(
                make_ticket("tr-0001", "Backend ticket"),
                &["backend", "api"],
            ),
            with_tags(make_ticket("tr-0002", "Frontend ticket"), &["frontend"]),
        ];
        let output = format_ready(&tickets, None, Some("backend"), None);
        assert!(
            output.contains("tr-0001"),
            "expected backend ticket in ready output\n{output}"
        );
        assert!(
            !output.contains("tr-0002"),
            "expected frontend ticket excluded from ready output\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Empty output when no tickets are ready
    // -----------------------------------------------------------------------

    #[test]
    fn ready_empty_output_when_no_tickets_ready() {
        let tickets = vec![
            with_status(make_ticket("tr-0001", "Closed A"), Status::Closed),
            with_status(make_ticket("tr-0002", "Closed B"), Status::Closed),
        ];
        let output = format_ready(&tickets, None, None, None);
        assert!(
            output.is_empty(),
            "expected empty output when no tickets are ready\n{output}"
        );
    }

    // =======================================================================
    // blocked command tests
    // =======================================================================

    // -----------------------------------------------------------------------
    // Ticket with any unclosed dep is blocked
    // -----------------------------------------------------------------------

    #[test]
    fn blocked_ticket_with_unclosed_dep_appears() {
        let tickets = vec![
            with_deps(make_ticket("block-001", "Blocked ticket"), &["block-002"]),
            make_ticket("block-002", "Blocker ticket"), // status defaults to open
        ];
        let output = format_blocked(&tickets, None, None, None);
        assert!(
            output.contains("block-001"),
            "expected blocked ticket in output\n{output}"
        );
        assert!(
            output.contains("[block-002]"),
            "expected blocker dep in output\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Ticket with all deps closed is not blocked
    // -----------------------------------------------------------------------

    #[test]
    fn blocked_ticket_with_all_deps_closed_excluded() {
        let tickets = vec![
            with_deps(make_ticket("block-001", "Unblocked ticket"), &["block-002"]),
            with_status(make_ticket("block-002", "Closed blocker"), Status::Closed),
        ];
        let output = format_blocked(&tickets, None, None, None);
        assert!(
            !output.contains("block-001"),
            "expected ticket with all closed deps to be excluded\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Ticket with no deps is not blocked
    // -----------------------------------------------------------------------

    #[test]
    fn blocked_ticket_with_no_deps_excluded() {
        let tickets = vec![make_ticket("block-001", "No deps ticket")];
        let output = format_blocked(&tickets, None, None, None);
        assert!(
            !output.contains("block-001"),
            "expected ticket with no deps to be excluded from blocked output\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Closed ticket is not blocked
    // -----------------------------------------------------------------------

    #[test]
    fn blocked_closed_ticket_excluded() {
        let tickets = vec![
            with_status(
                with_deps(make_ticket("block-001", "Closed blocked"), &["block-002"]),
                Status::Closed,
            ),
            make_ticket("block-002", "Open blocker"),
        ];
        let output = format_blocked(&tickets, None, None, None);
        assert!(
            !output.contains("block-001"),
            "expected closed ticket to be excluded from blocked output\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Only unclosed deps shown in output
    // -----------------------------------------------------------------------

    #[test]
    fn blocked_only_unclosed_deps_shown() {
        let tickets = vec![
            with_deps(
                make_ticket("block-001", "Blocked ticket"),
                &["block-002", "block-003"],
            ),
            make_ticket("block-002", "Open blocker"),
            with_status(make_ticket("block-003", "Closed blocker"), Status::Closed),
        ];
        let output = format_blocked(&tickets, None, None, None);
        assert!(
            output.contains("[block-002]"),
            "expected only open dep in output\n{output}"
        );
        assert!(
            !output.contains("block-003"),
            "expected closed dep excluded from output\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Priority badge format
    // -----------------------------------------------------------------------

    #[test]
    fn blocked_priority_badge_format() {
        let tickets = vec![
            with_deps(make_ticket("block-001", "Blocked ticket"), &["block-002"]),
            make_ticket("block-002", "Blocker"),
        ];
        let output = format_blocked(&tickets, None, None, None);
        let line = output.trim_end_matches('\n');
        // Format: {id} {priority} {status} {title} [{deps}]
        assert!(
            line.starts_with("block-001"),
            "expected id at start\n{line}"
        );
        assert!(line.contains("P2"), "expected priority 'P2'\n{line}");
        assert!(line.contains("open"), "expected status 'open'\n{line}");
        assert!(
            line.contains("Blocked ticket"),
            "expected title in line\n{line}"
        );
        assert!(line.contains("[block-002]"), "expected dep in line\n{line}");
    }

    // -----------------------------------------------------------------------
    // Sort by priority then ID
    // -----------------------------------------------------------------------

    #[test]
    fn blocked_sort_by_priority_then_id() {
        // Pin created to the same timestamp so ID is the stable tiebreaker.
        let t0 = chrono::DateTime::from_timestamp(0, 0).unwrap();
        let dep = with_status(make_ticket("dep", "Dep"), Status::Open);
        let tickets = vec![
            with_created(
                with_deps(with_priority(make_ticket("c", "Low priority"), 3), &["dep"]),
                t0,
            ),
            with_created(
                with_deps(
                    with_priority(make_ticket("b", "Also high priority"), 1),
                    &["dep"],
                ),
                t0,
            ),
            with_created(
                with_deps(
                    with_priority(make_ticket("a", "High priority"), 1),
                    &["dep"],
                ),
                t0,
            ),
            dep,
        ];
        let output = format_blocked(&tickets, None, None, None);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3, "expected 3 blocked lines\n{output}");
        assert!(
            lines[0].starts_with("a"),
            "expected 'a' first (priority 1, id a)\n{output}"
        );
        assert!(
            lines[1].starts_with("b"),
            "expected 'b' second (priority 1, id b)\n{output}"
        );
        assert!(
            lines[2].starts_with("c"),
            "expected 'c' third (priority 3)\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // -a/--assignee filter
    // -----------------------------------------------------------------------

    #[test]
    fn blocked_assignee_filter() {
        let tickets = vec![
            with_assignee(
                with_deps(make_ticket("block-001", "Alice blocked"), &["dep"]),
                "Alice",
            ),
            with_assignee(
                with_deps(make_ticket("block-002", "Bob blocked"), &["dep"]),
                "Bob",
            ),
            make_ticket("dep", "Open dep"),
        ];
        let output = format_blocked(&tickets, Some("Alice"), None, None);
        assert!(
            output.contains("block-001"),
            "expected Alice's ticket in blocked output\n{output}"
        );
        assert!(
            !output.contains("block-002"),
            "expected Bob's ticket excluded from blocked output\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // -T/--tag filter
    // -----------------------------------------------------------------------

    #[test]
    fn blocked_tag_filter() {
        let tickets = vec![
            with_tags(
                with_deps(make_ticket("block-001", "Backend blocked"), &["dep"]),
                &["backend"],
            ),
            with_tags(
                with_deps(make_ticket("block-002", "Frontend blocked"), &["dep"]),
                &["frontend"],
            ),
            make_ticket("dep", "Open dep"),
        ];
        let output = format_blocked(&tickets, None, Some("backend"), None);
        assert!(
            output.contains("block-001"),
            "expected backend ticket in blocked output\n{output}"
        );
        assert!(
            !output.contains("block-002"),
            "expected frontend ticket excluded from blocked output\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Empty output when no tickets are blocked
    // -----------------------------------------------------------------------

    #[test]
    fn blocked_empty_output_when_none_blocked() {
        let tickets = vec![
            make_ticket("tr-0001", "No deps"),
            with_status(make_ticket("tr-0002", "Closed"), Status::Closed),
            with_deps(
                make_ticket("tr-0003", "All deps closed"),
                &["tr-closed-dep"],
            ),
            with_status(make_ticket("tr-closed-dep", "Closed dep"), Status::Closed),
        ];
        let output = format_blocked(&tickets, None, None, None);
        assert!(
            output.is_empty(),
            "expected empty output when no tickets are blocked\n{output}"
        );
    }

    // =======================================================================
    // closed command tests
    // =======================================================================

    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Write a ticket file with given status to a temp tickets dir; return its path.
    fn write_closed_ticket_file(
        tickets_dir: &std::path::Path,
        id: &str,
        status: Status,
        title: &str,
        assignee: Option<&str>,
        tags: Option<&[&str]>,
    ) -> PathBuf {
        let assignee_line = assignee
            .map(|a| format!("\nassignee: {a}"))
            .unwrap_or_default();
        let tags_line = tags
            .map(|ts| {
                let joined = ts.join(", ");
                format!("\ntags: [{joined}]")
            })
            .unwrap_or_default();
        let content = format!(
            "---\nid: {id}\nstatus: {status}\ndeps: []\nlinks: []\ncreated: 2026-01-01T00:00:00Z\ntype: task\npriority: 2{assignee_line}{tags_line}\n---\n# {title}\n",
        );
        let path = tickets_dir.join(format!("{id}.md"));
        std::fs::write(&path, &content).unwrap();
        path
    }

    fn make_tickets_dir(root: &TempDir) -> std::path::PathBuf {
        let dir = root.path().join(".tickets");
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    // -----------------------------------------------------------------------
    // Shows closed tickets
    // -----------------------------------------------------------------------

    #[test]
    fn closed_shows_closed_ticket() {
        let root = tempfile::tempdir().unwrap();
        let tickets_dir = make_tickets_dir(&root);
        write_closed_ticket_file(
            &tickets_dir,
            "cl-0001",
            Status::Closed,
            "A closed ticket",
            None,
            None,
        );
        let output = closed_impl(Some(root.path()), 20, None, None, None).unwrap();
        assert!(
            output.contains("cl-0001"),
            "expected closed ticket id in output\n{output}"
        );
        assert!(
            output.contains("closed"),
            "expected 'closed' in output\n{output}"
        );
        assert!(
            output.contains("A closed ticket"),
            "expected title in output\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Excludes open tickets
    // -----------------------------------------------------------------------

    #[test]
    fn closed_excludes_open_tickets() {
        let root = tempfile::tempdir().unwrap();
        let tickets_dir = make_tickets_dir(&root);
        write_closed_ticket_file(
            &tickets_dir,
            "op-0001",
            Status::Open,
            "An open ticket",
            None,
            None,
        );
        let output = closed_impl(Some(root.path()), 20, None, None, None).unwrap();
        assert!(
            !output.contains("op-0001"),
            "expected open ticket to be excluded\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Output format
    // -----------------------------------------------------------------------

    #[test]
    fn closed_output_format() {
        let root = tempfile::tempdir().unwrap();
        let tickets_dir = make_tickets_dir(&root);
        write_closed_ticket_file(
            &tickets_dir,
            "fmt-0001",
            Status::Closed,
            "Format ticket",
            None,
            None,
        );
        let output = closed_impl(Some(root.path()), 20, None, None, None).unwrap();
        let line = output.trim_end_matches('\n');
        assert!(
            line.starts_with("fmt-0001"),
            "expected line to start with id\n{line}"
        );
        assert!(line.contains("closed"), "expected 'closed' in line\n{line}");
        assert!(
            line.contains("Format ticket"),
            "expected 'Format ticket' in line\n{line}"
        );
    }

    // -----------------------------------------------------------------------
    // --limit respected
    // -----------------------------------------------------------------------

    #[test]
    fn closed_limit_respected() {
        let root = tempfile::tempdir().unwrap();
        let tickets_dir = make_tickets_dir(&root);
        for i in 1..=3 {
            write_closed_ticket_file(
                &tickets_dir,
                &format!("lim-{i:04}"),
                Status::Closed,
                &format!("Ticket {i}"),
                None,
                None,
            );
        }
        let output = closed_impl(Some(root.path()), 1, None, None, None).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(
            lines.len(),
            1,
            "expected exactly 1 line with --limit=1\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Default limit of 20
    // -----------------------------------------------------------------------

    #[test]
    fn closed_default_limit_is_20() {
        let root = tempfile::tempdir().unwrap();
        let tickets_dir = make_tickets_dir(&root);
        for i in 1..=25 {
            write_closed_ticket_file(
                &tickets_dir,
                &format!("def-{i:04}"),
                Status::Closed,
                &format!("Ticket {i}"),
                None,
                None,
            );
        }
        let output = closed_impl(Some(root.path()), 20, None, None, None).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert!(
            lines.len() <= 20,
            "expected at most 20 lines with default limit, got {}\n{output}",
            lines.len()
        );
    }

    // -----------------------------------------------------------------------
    // Sorted by mtime (most recent first)
    // -----------------------------------------------------------------------

    #[test]
    fn closed_sorted_by_mtime_most_recent_first() {
        use filetime::FileTime;

        let root = tempfile::tempdir().unwrap();
        let tickets_dir = make_tickets_dir(&root);

        let path_older = write_closed_ticket_file(
            &tickets_dir,
            "mtime-older",
            Status::Closed,
            "Older ticket",
            None,
            None,
        );
        let path_newer = write_closed_ticket_file(
            &tickets_dir,
            "mtime-newer",
            Status::Closed,
            "Newer ticket",
            None,
            None,
        );

        // Set older file to a time in the past, newer file to a more recent time.
        filetime::set_file_mtime(&path_older, FileTime::from_unix_time(1000, 0)).unwrap();
        filetime::set_file_mtime(&path_newer, FileTime::from_unix_time(2000, 0)).unwrap();

        let output = closed_impl(Some(root.path()), 20, None, None, None).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2, "expected 2 lines\n{output}");
        assert!(
            lines[0].contains("mtime-newer"),
            "expected newer ticket first\n{output}"
        );
        assert!(
            lines[1].contains("mtime-older"),
            "expected older ticket second\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // -a assignee filter
    // -----------------------------------------------------------------------

    #[test]
    fn closed_assignee_filter() {
        let root = tempfile::tempdir().unwrap();
        let tickets_dir = make_tickets_dir(&root);
        write_closed_ticket_file(
            &tickets_dir,
            "cl-alice",
            Status::Closed,
            "Alice ticket",
            Some("Alice"),
            None,
        );
        write_closed_ticket_file(
            &tickets_dir,
            "cl-bob",
            Status::Closed,
            "Bob ticket",
            Some("Bob"),
            None,
        );
        let output = closed_impl(Some(root.path()), 20, Some("Alice"), None, None).unwrap();
        assert!(
            output.contains("cl-alice"),
            "expected Alice's ticket\n{output}"
        );
        assert!(
            !output.contains("cl-bob"),
            "expected Bob's ticket excluded\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // -T tag filter
    // -----------------------------------------------------------------------

    #[test]
    fn closed_tag_filter() {
        let root = tempfile::tempdir().unwrap();
        let tickets_dir = make_tickets_dir(&root);
        write_closed_ticket_file(
            &tickets_dir,
            "cl-back",
            Status::Closed,
            "Backend ticket",
            None,
            Some(&["backend"]),
        );
        write_closed_ticket_file(
            &tickets_dir,
            "cl-front",
            Status::Closed,
            "Frontend ticket",
            None,
            Some(&["frontend"]),
        );
        let output = closed_impl(Some(root.path()), 20, None, Some("backend"), None).unwrap();
        assert!(
            output.contains("cl-back"),
            "expected backend ticket\n{output}"
        );
        assert!(
            !output.contains("cl-front"),
            "expected frontend ticket excluded\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Empty output when no closed tickets
    // -----------------------------------------------------------------------

    #[test]
    fn closed_empty_output_when_no_closed_tickets() {
        let root = tempfile::tempdir().unwrap();
        let tickets_dir = make_tickets_dir(&root);
        write_closed_ticket_file(
            &tickets_dir,
            "op-only",
            Status::Open,
            "Open ticket",
            None,
            None,
        );
        let output = closed_impl(Some(root.path()), 20, None, None, None).unwrap();
        assert!(
            output.is_empty(),
            "expected empty output when no closed tickets\n{output}"
        );
    }
}
