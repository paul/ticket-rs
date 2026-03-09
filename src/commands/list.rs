// Implementation of the `ls` / `list` and `ready` subcommands.

use std::collections::HashMap;
use std::path::Path;

use crate::error::Result;
use crate::store::TicketStore;
use crate::ticket::{Status, Ticket};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// List tickets, optionally filtered by status, assignee, and/or tag.
pub fn ls(status: Option<&str>, assignee: Option<&str>, tag: Option<&str>) -> Result<()> {
    let output = ls_impl(None, status, assignee, tag)?;
    print!("{output}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal implementation (testable via explicit start_dir)
// ---------------------------------------------------------------------------

/// Core logic for `ls`.
///
/// `start_dir` is passed to `TicketStore::find`; `None` uses the cwd.
/// Returns the full formatted output string (empty when no tickets match).
fn ls_impl(
    start_dir: Option<&Path>,
    status: Option<&str>,
    assignee: Option<&str>,
    tag: Option<&str>,
) -> Result<String> {
    let store = TicketStore::find(start_dir)?;
    let tickets = store.list_tickets();
    let output = format_list(&tickets, status, assignee, tag)?;
    Ok(output)
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
) -> Result<String> {
    // Parse the status filter once up-front so we can return an error early.
    let status_filter: Option<Status> = status.map(|s| s.parse::<Status>()).transpose()?;

    // Apply filters.
    let mut filtered: Vec<&Ticket> = tickets
        .iter()
        .filter(|t| {
            if let Some(ref s) = status_filter
                && &t.status != s
            {
                return false;
            }
            if let Some(a) = assignee
                && t.assignee.as_deref() != Some(a)
            {
                return false;
            }
            if let Some(tag_val) = tag {
                let has_tag = t
                    .tags
                    .as_ref()
                    .map(|tags| tags.iter().any(|t| t == tag_val))
                    .unwrap_or(false);
                if !has_tag {
                    return false;
                }
            }
            true
        })
        .collect();

    // Sort by priority ascending, then by ID ascending.
    filtered.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.id.cmp(&b.id)));

    // Format each ticket line.
    let lines: Vec<String> = filtered.iter().map(|t| format_line(t)).collect();

    if lines.is_empty() {
        return Ok(String::new());
    }

    Ok(lines.join("\n") + "\n")
}

/// Format a single ticket as a display line.
///
/// Format: `{id:<12}  [{status}] - {title}` with an optional `  <- [{deps}]`
/// suffix when the ticket has dependencies.
fn format_line(ticket: &Ticket) -> String {
    let mut line = format!("{:<12}  [{}] - {}", ticket.id, ticket.status, ticket.title);
    if !ticket.deps.is_empty() {
        line.push_str(&format!("  <- [{}]", ticket.deps.join(", ")));
    }
    line
}

// ---------------------------------------------------------------------------
// ready — public entry point
// ---------------------------------------------------------------------------

/// Show tickets that are ready to work on (open or in-progress, all deps closed).
pub fn ready(assignee: Option<&str>, tag: Option<&str>) -> Result<()> {
    let output = ready_impl(None, assignee, tag)?;
    print!("{output}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal implementation (testable via explicit start_dir)
// ---------------------------------------------------------------------------

/// Core logic for `ready`.
///
/// `start_dir` is passed to `TicketStore::find`; `None` uses the cwd.
/// Returns the full formatted output string (empty when no tickets match).
fn ready_impl(
    start_dir: Option<&Path>,
    assignee: Option<&str>,
    tag: Option<&str>,
) -> Result<String> {
    let store = TicketStore::find(start_dir)?;
    let tickets = store.list_tickets();
    let output = format_ready(&tickets, assignee, tag);
    Ok(output)
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
) -> String {
    // Build a lookup map so dependency checks are O(1).
    let by_id: HashMap<&str, &Ticket> = tickets.iter().map(|t| (t.id.as_str(), t)).collect();

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
                .all(|dep_id| matches!(by_id.get(dep_id.as_str()), Some(d) if d.status == Status::Closed))
            {
                return false;
            }
            // Optional assignee filter.
            if let Some(a) = assignee
                && t.assignee.as_deref() != Some(a)
            {
                return false;
            }
            // Optional tag filter.
            if let Some(tag_val) = tag {
                let has_tag = t
                    .tags
                    .as_ref()
                    .map(|tags| tags.iter().any(|tg| tg == tag_val))
                    .unwrap_or(false);
                if !has_tag {
                    return false;
                }
            }
            true
        })
        .collect();

    // Sort by priority ascending, then by ID ascending.
    filtered.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.id.cmp(&b.id)));

    let lines: Vec<String> = filtered.iter().map(|t| format_ready_line(t)).collect();

    if lines.is_empty() {
        return String::new();
    }

    lines.join("\n") + "\n"
}

/// Format a single ticket as a ready-list display line.
///
/// Format: `{id:<12}  [P{priority}][{status}] - {title}`
fn format_ready_line(ticket: &Ticket) -> String {
    format!(
        "{:<12}  [P{}][{}] - {}",
        ticket.id, ticket.priority, ticket.status, ticket.title
    )
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

    // -----------------------------------------------------------------------
    // Lists all tickets
    // -----------------------------------------------------------------------

    #[test]
    fn lists_all_tickets() {
        let tickets = vec![
            make_ticket("tr-0001", "First"),
            make_ticket("tr-0002", "Second"),
        ];
        let output = format_list(&tickets, None, None, None).unwrap();
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
        let output = format_list(&tickets, None, None, None).unwrap();
        // Must match: ID  [STATUS] - TITLE (with at least one space between ID and [STATUS])
        let line = output.trim_end_matches('\n');
        assert!(line.contains("[open]"), "expected '[open]' in line\n{line}");
        assert!(
            line.contains("- My ticket"),
            "expected '- My ticket' in line\n{line}"
        );
        // ID appears at the start of the line
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
        let output = format_list(&tickets, None, None, None).unwrap();
        assert!(
            output.contains("<- [dep-001]"),
            "expected '<- [dep-001]' in output\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Deps hidden when empty
    // -----------------------------------------------------------------------

    #[test]
    fn deps_hidden_when_empty() {
        let tickets = vec![make_ticket("tr-0001", "No deps")];
        let output = format_list(&tickets, None, None, None).unwrap();
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
        let output = format_list(&tickets, Some("open"), None, None).unwrap();
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
        let output = format_list(&tickets, Some("open"), None, None).unwrap();
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
        let output = format_list(&tickets, None, Some("Alice"), None).unwrap();
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
        let output = format_list(&tickets, None, None, Some("backend")).unwrap();
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
        let tickets = vec![
            with_priority(make_ticket("c", "Low priority"), 3),
            with_priority(make_ticket("b", "High priority B"), 1),
            with_priority(make_ticket("a", "High priority A"), 1),
        ];
        let output = format_list(&tickets, None, None, None).unwrap();
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
        let output = format_list(&tickets, None, None, None).unwrap();
        assert!(
            output.is_empty(),
            "expected empty output for empty ticket list"
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
        let output = format_ready(&tickets, None, None);
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
        let output = format_ready(&tickets, None, None);
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
        let output = format_ready(&tickets, None, None);
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
        let output = format_ready(&tickets, None, None);
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
        let output = format_ready(&tickets, None, None);
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
        let output = format_ready(&tickets, None, None);
        let line = output.trim_end_matches('\n');
        // Assert exact line shape: "{id:<12}  [P{priority}][{status}] - {title}"
        assert_eq!(
            line,
            "ready-001     [P2][open] - Priority ticket",
            "output line did not match expected format"
        );
    }

    // -----------------------------------------------------------------------
    // Sort by priority then ID
    // -----------------------------------------------------------------------

    #[test]
    fn ready_sort_by_priority_then_id() {
        let tickets = vec![
            with_priority(make_ticket("c", "Low priority"), 3),
            with_priority(make_ticket("b", "Also high priority"), 1),
            with_priority(make_ticket("a", "High priority"), 1),
        ];
        let output = format_ready(&tickets, None, None);
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
        let output = format_ready(&tickets, Some("Alice"), None);
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
        let output = format_ready(&tickets, None, Some("backend"));
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
        let output = format_ready(&tickets, None, None);
        assert!(
            output.is_empty(),
            "expected empty output when no tickets are ready\n{output}"
        );
    }
}
