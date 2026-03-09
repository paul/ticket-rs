// Implementation of the `ls` / `list` subcommand.

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
            if let Some(ref s) = status_filter {
                if &t.status != s {
                    return false;
                }
            }
            if let Some(a) = assignee {
                if t.assignee.as_deref() != Some(a) {
                    return false;
                }
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
}
