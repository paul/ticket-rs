// Implementation of the `search` subcommand.
//
// Performs a case-insensitive substring match against each ticket's title and
// body.  Closed tickets are excluded by default; pass `all = true` to include
// them.  The optional `status`, `assignee`, and `tag` filters work identically
// to those on `ls`.
//
// Output format is identical to `ls`: one ticket per line using `ticket_line`.

use std::collections::HashMap;
use std::path::Path;

use crate::commands::list::{empty_dir_message, ticket_line, tty_width};
use crate::error::Result;
use crate::pager;
use crate::store::TicketStore;
use crate::ticket::{Status, Ticket};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Search tickets by case-insensitive substring match on title and body.
pub fn search(
    query: &str,
    all: bool,
    status: Option<&str>,
    assignee: Option<&str>,
    tag: Option<&str>,
) -> Result<()> {
    let output = search_impl(None, query, all, status, assignee, tag, tty_width())?;
    pager::page_or_print(&output)
}

// ---------------------------------------------------------------------------
// Core logic (separated for testability)
// ---------------------------------------------------------------------------

fn search_impl(
    start_dir: Option<&Path>,
    query: &str,
    all: bool,
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
    format_search(&tickets, query, all, status, assignee, tag, term_width)
}

/// Apply the search filter, optional extra filters, sort, and format the
/// output string.
///
/// Extracted so that unit tests can call it directly with in-memory ticket
/// slices without touching the filesystem.
pub(crate) fn format_search(
    tickets: &[Ticket],
    query: &str,
    all: bool,
    status: Option<&str>,
    assignee: Option<&str>,
    tag: Option<&str>,
    term_width: Option<usize>,
) -> Result<String> {
    let query_lower = query.to_lowercase();

    // Parse the status filter once up-front so we can return an error early.
    let status_filter: Option<Status> = status.map(|s| s.parse::<Status>()).transpose()?;

    // Build a status lookup map for dep coloring.
    let dep_statuses: HashMap<String, Status> = tickets
        .iter()
        .map(|t| (t.id.clone(), t.status.clone()))
        .collect();

    let mut filtered: Vec<&Ticket> = tickets
        .iter()
        .filter(|t| {
            // Text match: title or body must contain the query.
            let title_match = t.title.to_lowercase().contains(&query_lower);
            let body_match = t.body.to_lowercase().contains(&query_lower);
            if !title_match && !body_match {
                return false;
            }

            // Unless --all is set or an explicit --status filter is provided,
            // exclude closed tickets so the default view focuses on active work.
            if !all && status_filter.is_none() && t.status == Status::Closed {
                return false;
            }

            // Explicit status filter.
            if let Some(ref s) = status_filter
                && &t.status != s
            {
                return false;
            }

            // Assignee and tag filters.
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
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    use crate::ticket::{Status, Ticket, TicketType};

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

    fn with_body(mut t: Ticket, body: &str) -> Ticket {
        t.body = body.to_string();
        t
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

    // -----------------------------------------------------------------------
    // Matches title
    // -----------------------------------------------------------------------

    #[test]
    fn matches_title_substring() {
        let tickets = vec![
            make_ticket("tr-0001", "Export CSV data"),
            make_ticket("tr-0002", "Import JSON data"),
        ];
        let output = format_search(&tickets, "csv", false, None, None, None, None).unwrap();
        assert!(
            output.contains("tr-0001"),
            "expected csv title match\n{output}"
        );
        assert!(
            !output.contains("tr-0002"),
            "expected json title excluded\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Matches body
    // -----------------------------------------------------------------------

    #[test]
    fn matches_body_substring() {
        let tickets = vec![
            with_body(
                make_ticket("tr-0001", "Data export"),
                "# Data export\n\nSupports CSV format.\n",
            ),
            make_ticket("tr-0002", "Other ticket"),
        ];
        let output = format_search(&tickets, "csv", false, None, None, None, None).unwrap();
        assert!(output.contains("tr-0001"), "expected body match\n{output}");
        assert!(
            !output.contains("tr-0002"),
            "expected non-matching ticket excluded\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Case-insensitive
    // -----------------------------------------------------------------------

    #[test]
    fn case_insensitive_title_match() {
        let tickets = vec![make_ticket("tr-0001", "Export CSV Data")];
        for q in ["csv", "CSV", "Csv", "cSv"] {
            let output = format_search(&tickets, q, false, None, None, None, None).unwrap();
            assert!(
                output.contains("tr-0001"),
                "expected match for query '{q}'\n{output}"
            );
        }
    }

    #[test]
    fn case_insensitive_body_match() {
        let tickets = vec![with_body(
            make_ticket("tr-0001", "Export"),
            "# Export\n\nUses CSV format.\n",
        )];
        for q in ["csv", "CSV", "Csv"] {
            let output = format_search(&tickets, q, false, None, None, None, None).unwrap();
            assert!(
                output.contains("tr-0001"),
                "expected body match for query '{q}'\n{output}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Default excludes closed tickets
    // -----------------------------------------------------------------------

    #[test]
    fn default_excludes_closed() {
        let tickets = vec![
            with_status(make_ticket("tr-0001", "CSV export closed"), Status::Closed),
            make_ticket("tr-0002", "CSV export open"),
        ];
        let output = format_search(&tickets, "csv", false, None, None, None, None).unwrap();
        assert!(
            !output.contains("tr-0001"),
            "expected closed ticket excluded by default\n{output}"
        );
        assert!(
            output.contains("tr-0002"),
            "expected open ticket included\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // --all includes closed tickets
    // -----------------------------------------------------------------------

    #[test]
    fn all_flag_includes_closed() {
        let tickets = vec![
            with_status(make_ticket("tr-0001", "CSV export closed"), Status::Closed),
            make_ticket("tr-0002", "CSV export open"),
        ];
        let output = format_search(&tickets, "csv", true, None, None, None, None).unwrap();
        assert!(
            output.contains("tr-0001"),
            "expected closed ticket included with --all\n{output}"
        );
        assert!(
            output.contains("tr-0002"),
            "expected open ticket included with --all\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // --status filter
    // -----------------------------------------------------------------------

    #[test]
    fn status_filter_open_only() {
        let tickets = vec![
            make_ticket("tr-0001", "CSV open"),
            with_status(make_ticket("tr-0002", "CSV closed"), Status::Closed),
        ];
        let output = format_search(&tickets, "csv", false, Some("open"), None, None, None).unwrap();
        assert!(output.contains("tr-0001"), "expected open ticket\n{output}");
        assert!(
            !output.contains("tr-0002"),
            "expected closed ticket excluded\n{output}"
        );
    }

    #[test]
    fn status_filter_closed_shows_closed_without_all() {
        // Explicitly passing --status=closed should show closed tickets even
        // without --all.
        let tickets = vec![
            make_ticket("tr-0001", "CSV open"),
            with_status(make_ticket("tr-0002", "CSV closed"), Status::Closed),
        ];
        let output =
            format_search(&tickets, "csv", false, Some("closed"), None, None, None).unwrap();
        assert!(
            !output.contains("tr-0001"),
            "expected open ticket excluded\n{output}"
        );
        assert!(
            output.contains("tr-0002"),
            "expected closed ticket shown\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // --assignee filter
    // -----------------------------------------------------------------------

    #[test]
    fn assignee_filter() {
        let tickets = vec![
            with_assignee(make_ticket("tr-0001", "CSV for Alice"), "Alice"),
            with_assignee(make_ticket("tr-0002", "CSV for Bob"), "Bob"),
        ];
        let output =
            format_search(&tickets, "csv", false, None, Some("Alice"), None, None).unwrap();
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
    // --tag filter
    // -----------------------------------------------------------------------

    #[test]
    fn tag_filter() {
        let tickets = vec![
            with_tags(make_ticket("tr-0001", "CSV backend"), &["backend"]),
            with_tags(make_ticket("tr-0002", "CSV frontend"), &["frontend"]),
        ];
        let output =
            format_search(&tickets, "csv", false, None, None, Some("backend"), None).unwrap();
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
    // No match produces empty output
    // -----------------------------------------------------------------------

    #[test]
    fn no_match_produces_empty_output() {
        let tickets = vec![make_ticket("tr-0001", "Something unrelated")];
        let output = format_search(&tickets, "csv", false, None, None, None, None).unwrap();
        assert!(
            output.is_empty(),
            "expected empty output for no match\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Empty ticket list produces empty output (format_search level)
    // -----------------------------------------------------------------------

    #[test]
    fn empty_tickets_produces_empty_output() {
        let tickets: Vec<Ticket> = vec![];
        let output = format_search(&tickets, "csv", false, None, None, None, None).unwrap();
        assert!(output.is_empty(), "expected empty output for empty list");
    }

    // -----------------------------------------------------------------------
    // Empty store (no ticket files) shows the same message as ls
    // -----------------------------------------------------------------------

    #[test]
    fn empty_store_shows_empty_dir_message() {
        let root = tempfile::tempdir().unwrap();
        let tickets_dir = root.path().join(".tickets");
        std::fs::create_dir_all(&tickets_dir).unwrap();

        let output = search_impl(Some(root.path()), "csv", false, None, None, None, None).unwrap();
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

    // -----------------------------------------------------------------------
    // Output format matches ls
    // -----------------------------------------------------------------------

    #[test]
    fn output_format_matches_ls() {
        let tickets = vec![make_ticket("srch-0001", "CSV export feature")];
        let output = format_search(&tickets, "csv", false, None, None, None, None).unwrap();
        let line = output.trim_end_matches('\n');
        assert!(
            line.starts_with("srch-0001"),
            "expected id at start\n{line}"
        );
        assert!(line.contains("P2"), "expected priority\n{line}");
        assert!(line.contains("open"), "expected status\n{line}");
        assert!(
            line.contains("CSV export feature"),
            "expected title\n{line}"
        );
    }
}
