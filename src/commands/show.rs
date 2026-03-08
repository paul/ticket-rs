// Implementation of the `show` subcommand.

use std::path::Path;

use crate::error::Result;
use crate::store::TicketStore;
use crate::ticket::{Status, Ticket};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Display a ticket's full content, including dynamically computed sections.
pub fn show(id: &str) -> Result<()> {
    let output = show_impl(None, id)?;
    print!("{output}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal implementation (testable via explicit start_dir)
// ---------------------------------------------------------------------------

/// Core logic for `show`.
///
/// `start_dir` is the directory from which to locate `.tickets/`. Passing
/// `None` uses the current working directory. Tests pass `Some(tempdir.path())`
/// to avoid touching the real filesystem.
///
/// Returns the formatted output string ready to be printed.
fn show_impl(start_dir: Option<&Path>, id: &str) -> Result<String> {
    let store = TicketStore::find(start_dir)?;

    // Resolve the (possibly partial) ID to a full ticket ID.
    let path = store.resolve_id(id)?;
    let full_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(id)
        .to_string();

    let ticket = store.read_ticket(&full_id)?;
    let output = build_output(&ticket, &store);
    Ok(output)
}

// ---------------------------------------------------------------------------
// Output builder
// ---------------------------------------------------------------------------

/// Assemble the display string for a ticket.
///
/// The output is structured as:
/// 1. Frontmatter block (with optional inline parent title annotation).
/// 2. The raw markdown body (verbatim from disk).
/// 3. Dynamic sections: `## Blockers`, `## Blocking`, `## Children`, `## Linked`.
///    Each section is only included when it has at least one entry.
fn build_output(ticket: &Ticket, store: &TicketStore) -> String {
    let all_tickets = store.list_tickets();
    let mut out = String::new();

    // --- Frontmatter --------------------------------------------------------

    out.push_str("---\n");
    out.push_str(&format!("id: {}\n", ticket.id));
    out.push_str(&format!("status: {}\n", ticket.status));
    out.push_str(&format!("deps: [{}]\n", ticket.deps.join(", ")));
    out.push_str(&format!("links: [{}]\n", ticket.links.join(", ")));
    out.push_str(&format!(
        "created: {}\n",
        ticket.created.format("%Y-%m-%dT%H:%M:%SZ")
    ));
    out.push_str(&format!("type: {}\n", ticket.ticket_type));
    out.push_str(&format!("priority: {}\n", ticket.priority));
    if let Some(ref assignee) = ticket.assignee {
        out.push_str(&format!("assignee: {}\n", assignee));
    }
    if let Some(ref ext_ref) = ticket.external_ref {
        out.push_str(&format!("external-ref: {}\n", ext_ref));
    }
    if let Some(ref parent_id) = ticket.parent {
        // Annotate the parent field with the parent ticket's title when it can
        // be resolved; fall back to just the bare ID if the parent is missing.
        let annotation = all_tickets
            .iter()
            .find(|t| t.id == *parent_id)
            .map(|t| format!("  # {}", t.title));
        match annotation {
            Some(ann) => out.push_str(&format!("parent: {}{}\n", parent_id, ann)),
            None => out.push_str(&format!("parent: {}\n", parent_id)),
        }
    }
    if let Some(ref tags) = ticket.tags {
        out.push_str(&format!("tags: [{}]\n", tags.join(", ")));
    }
    out.push_str("---\n");

    // --- Body ---------------------------------------------------------------

    out.push_str(&ticket.body);

    // --- Dynamic sections ---------------------------------------------------

    // ## Blockers: deps that are not yet closed.
    let blockers: Vec<&Ticket> = ticket
        .deps
        .iter()
        .filter_map(|dep_id| all_tickets.iter().find(|t| t.id == *dep_id))
        .filter(|t| t.status != Status::Closed)
        .collect();

    if !blockers.is_empty() {
        out.push_str("\n## Blockers\n\n");
        for t in &blockers {
            out.push_str(&format!("- {} [{}] {}\n", t.id, t.status, t.title));
        }
    }

    // ## Blocking: tickets that list this one in their deps and are not closed.
    let blocking: Vec<&Ticket> = all_tickets
        .iter()
        .filter(|t| t.id != ticket.id && t.deps.contains(&ticket.id))
        .collect();

    if !blocking.is_empty() {
        out.push_str("\n## Blocking\n\n");
        for t in blocking {
            out.push_str(&format!("- {} [{}] {}\n", t.id, t.status, t.title));
        }
    }

    // ## Children: tickets whose parent field points to this ticket.
    let children: Vec<&Ticket> = all_tickets
        .iter()
        .filter(|t| t.parent.as_deref() == Some(ticket.id.as_str()))
        .collect();

    if !children.is_empty() {
        out.push_str("\n## Children\n\n");
        for t in children {
            out.push_str(&format!("- {} [{}] {}\n", t.id, t.status, t.title));
        }
    }

    // ## Linked: tickets referenced in this ticket's links array.
    let linked: Vec<&Ticket> = ticket
        .links
        .iter()
        .filter_map(|link_id| all_tickets.iter().find(|t| t.id == *link_id))
        .collect();

    if !linked.is_empty() {
        out.push_str("\n## Linked\n\n");
        for t in linked {
            out.push_str(&format!("- {} [{}] {}\n", t.id, t.status, t.title));
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Write a minimal ticket file into `tickets_dir`.
    fn write_ticket(tickets_dir: &Path, id: &str, title: &str) {
        let content = format!(
            "---\nid: {id}\nstatus: open\ndeps: []\nlinks: []\ncreated: 2026-01-01T00:00:00Z\ntype: task\npriority: 2\n---\n# {title}\n\nDescription\n"
        );
        std::fs::write(tickets_dir.join(format!("{id}.md")), content).unwrap();
    }

    /// Write a ticket with a custom status.
    fn write_ticket_with_status(tickets_dir: &Path, id: &str, title: &str, status: &str) {
        let content = format!(
            "---\nid: {id}\nstatus: {status}\ndeps: []\nlinks: []\ncreated: 2026-01-01T00:00:00Z\ntype: task\npriority: 2\n---\n# {title}\n\nDescription\n"
        );
        std::fs::write(tickets_dir.join(format!("{id}.md")), content).unwrap();
    }

    /// Write a ticket with a dep list.
    fn write_ticket_with_deps(tickets_dir: &Path, id: &str, title: &str, deps: &[&str]) {
        let deps_str = deps.join(", ");
        let content = format!(
            "---\nid: {id}\nstatus: open\ndeps: [{deps_str}]\nlinks: []\ncreated: 2026-01-01T00:00:00Z\ntype: task\npriority: 2\n---\n# {title}\n\nDescription\n"
        );
        std::fs::write(tickets_dir.join(format!("{id}.md")), content).unwrap();
    }

    /// Write a ticket with a links list.
    fn write_ticket_with_links(tickets_dir: &Path, id: &str, title: &str, links: &[&str]) {
        let links_str = links.join(", ");
        let content = format!(
            "---\nid: {id}\nstatus: open\ndeps: []\nlinks: [{links_str}]\ncreated: 2026-01-01T00:00:00Z\ntype: task\npriority: 2\n---\n# {title}\n\nDescription\n"
        );
        std::fs::write(tickets_dir.join(format!("{id}.md")), content).unwrap();
    }

    /// Write a ticket with a parent field.
    fn write_ticket_with_parent(tickets_dir: &Path, id: &str, title: &str, parent: &str) {
        let content = format!(
            "---\nid: {id}\nstatus: open\ndeps: []\nlinks: []\ncreated: 2026-01-01T00:00:00Z\ntype: task\npriority: 2\nparent: {parent}\n---\n# {title}\n\nDescription\n"
        );
        std::fs::write(tickets_dir.join(format!("{id}.md")), content).unwrap();
    }

    /// Create a store backed by a fresh `.tickets/` dir inside `root`.
    fn make_store(root: &TempDir) -> TicketStore {
        let dir = root.path().join(".tickets");
        std::fs::create_dir_all(&dir).unwrap();
        TicketStore::find(Some(root.path())).unwrap()
    }

    // -----------------------------------------------------------------------
    // Displays raw content
    // -----------------------------------------------------------------------

    #[test]
    fn displays_raw_content() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);
        write_ticket(store.dir(), "show-001", "Test ticket");

        let output = show_impl(Some(root.path()), "show-001").unwrap();
        assert!(
            output.contains("id: show-001"),
            "expected 'id: show-001' in output\n{output}"
        );
        assert!(
            output.contains("# Test ticket"),
            "expected '# Test ticket' in output\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // All frontmatter fields shown
    // -----------------------------------------------------------------------

    #[test]
    fn all_frontmatter_fields_shown() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);
        write_ticket(store.dir(), "show-001", "Full ticket");

        let output = show_impl(Some(root.path()), "show-001").unwrap();
        assert!(output.contains("status: open"), "missing status\n{output}");
        assert!(output.contains("deps: []"), "missing deps\n{output}");
        assert!(output.contains("links: []"), "missing links\n{output}");
        assert!(output.contains("type: task"), "missing type\n{output}");
        assert!(output.contains("priority: 2"), "missing priority\n{output}");
    }

    // -----------------------------------------------------------------------
    // ## Blockers — present when deps unclosed
    // -----------------------------------------------------------------------

    #[test]
    fn blockers_section_present_when_deps_unclosed() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);
        write_ticket(store.dir(), "show-002", "Blocker ticket");
        write_ticket_with_deps(store.dir(), "show-001", "Blocked ticket", &["show-002"]);

        let output = show_impl(Some(root.path()), "show-001").unwrap();
        assert!(
            output.contains("## Blockers"),
            "expected '## Blockers' section\n{output}"
        );
        assert!(
            output.contains("show-002"),
            "expected dep id in Blockers\n{output}"
        );
        assert!(
            output.contains("Blocker ticket"),
            "expected dep title in Blockers\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // ## Blockers — absent when all deps closed
    // -----------------------------------------------------------------------

    #[test]
    fn blockers_section_absent_when_all_deps_closed() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);
        write_ticket_with_status(store.dir(), "show-002", "Closed blocker", "closed");
        write_ticket_with_deps(store.dir(), "show-001", "Unblocked ticket", &["show-002"]);

        let output = show_impl(Some(root.path()), "show-001").unwrap();
        assert!(
            !output.contains("## Blockers"),
            "expected no '## Blockers' section when all deps are closed\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // ## Blocking section
    // -----------------------------------------------------------------------

    #[test]
    fn blocking_section_shows_tickets_that_depend_on_this() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);
        write_ticket(store.dir(), "show-001", "Blocker");
        write_ticket_with_deps(store.dir(), "show-002", "Blocked", &["show-001"]);

        let output = show_impl(Some(root.path()), "show-001").unwrap();
        assert!(
            output.contains("## Blocking"),
            "expected '## Blocking' section\n{output}"
        );
        assert!(
            output.contains("show-002"),
            "expected blocked ticket id in Blocking\n{output}"
        );
        assert!(
            output.contains("Blocked"),
            "expected blocked ticket title in Blocking\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // ## Children section
    // -----------------------------------------------------------------------

    #[test]
    fn children_section_shows_child_tickets() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);
        write_ticket(store.dir(), "show-001", "Parent");
        write_ticket_with_parent(store.dir(), "show-002", "Child", "show-001");

        let output = show_impl(Some(root.path()), "show-001").unwrap();
        assert!(
            output.contains("## Children"),
            "expected '## Children' section\n{output}"
        );
        assert!(
            output.contains("show-002"),
            "expected child id in Children\n{output}"
        );
        assert!(
            output.contains("Child"),
            "expected child title in Children\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // ## Linked section
    // -----------------------------------------------------------------------

    #[test]
    fn linked_section_shows_linked_tickets() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);
        write_ticket(store.dir(), "show-002", "Second");
        write_ticket_with_links(store.dir(), "show-001", "First", &["show-002"]);

        let output = show_impl(Some(root.path()), "show-001").unwrap();
        assert!(
            output.contains("## Linked"),
            "expected '## Linked' section\n{output}"
        );
        assert!(
            output.contains("show-002"),
            "expected linked ticket id in Linked\n{output}"
        );
        assert!(
            output.contains("[open]"),
            "expected status in Linked entry\n{output}"
        );
        assert!(
            output.contains("Second"),
            "expected linked ticket title in Linked\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Parent annotation
    // -----------------------------------------------------------------------

    #[test]
    fn parent_annotation_shows_parent_title() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);
        write_ticket(store.dir(), "show-001", "Parent ticket");
        write_ticket_with_parent(store.dir(), "show-002", "Child ticket", "show-001");

        let output = show_impl(Some(root.path()), "show-002").unwrap();
        assert!(
            output.contains("parent: show-001"),
            "expected 'parent: show-001' in output\n{output}"
        );
        assert!(
            output.contains("# Parent ticket"),
            "expected parent title annotation in output\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Non-existent ticket
    // -----------------------------------------------------------------------

    #[test]
    fn nonexistent_ticket_returns_error() {
        let root = tempfile::tempdir().unwrap();
        let _store = make_store(&root);

        let err = show_impl(Some(root.path()), "nonexistent").unwrap_err();
        assert!(
            matches!(err, crate::error::Error::TicketNotFound { .. }),
            "expected TicketNotFound, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Partial ID resolution
    // -----------------------------------------------------------------------

    #[test]
    fn partial_id_resolution() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);
        write_ticket(store.dir(), "show-001", "Test ticket");

        let output = show_impl(Some(root.path()), "001").unwrap();
        assert!(
            output.contains("id: show-001"),
            "expected 'id: show-001' in output via partial ID\n{output}"
        );
    }
}
