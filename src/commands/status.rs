// Status commands: start, close, reopen, status.
//
// All four commands resolve a partial ID, read the ticket, update the status
// field, and write the ticket back using the existing round-trip serializer.
// The round-trip is byte-identical for all fields except status, so the body
// and all other frontmatter fields are preserved exactly.

use std::path::Path;

use crate::error::Result;
use crate::store::TicketStore;
use crate::ticket::Status;

/// Resolve the ticket, update its status, write it back, and return the output
/// message to be printed (`"Updated <id> -> <status>"`).
fn set_status_impl(
    start_dir: Option<&Path>,
    partial_id: &str,
    new_status: Status,
) -> Result<String> {
    let store = TicketStore::find(start_dir)?;

    let path = store.resolve_id(partial_id)?;
    let full_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(partial_id)
        .to_string();

    let mut ticket = store.read_ticket(&full_id)?;
    ticket.status = new_status;
    store.write_ticket(&ticket)?;

    Ok(format!("Updated {full_id} -> {}", ticket.status))
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

pub fn start(id: &str) -> Result<()> {
    let msg = set_status_impl(None, id, Status::InProgress)?;
    println!("{msg}");
    Ok(())
}

pub fn close(id: &str) -> Result<()> {
    let msg = set_status_impl(None, id, Status::Closed)?;
    println!("{msg}");
    Ok(())
}

pub fn reopen(id: &str) -> Result<()> {
    let msg = set_status_impl(None, id, Status::Open)?;
    println!("{msg}");
    Ok(())
}

pub fn status(id: &str, status_str: &str) -> Result<()> {
    let new_status = status_str.parse::<Status>()?;
    let msg = set_status_impl(None, id, new_status)?;
    println!("{msg}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use std::fs;
    use tempfile::tempdir;

    /// Write a minimal ticket file to a temp .tickets/ dir, returning the store.
    fn make_store_with_ticket(dir: &Path, id: &str, status: &str, extra_body: &str) -> TicketStore {
        let tickets_dir = dir.join(".tickets");
        fs::create_dir_all(&tickets_dir).unwrap();

        let content = format!(
            "---\nid: {id}\nstatus: {status}\ndeps: []\nlinks: []\ncreated: 2026-01-01T00:00:00Z\ntype: task\npriority: 2\n---\n# Test ticket\n{extra_body}"
        );
        fs::write(tickets_dir.join(format!("{id}.md")), &content).unwrap();

        // Point the store at the temp dir via TICKETS_DIR so find() works
        // without cwd manipulation.
        TicketStore::find(Some(dir)).unwrap()
    }

    fn read_status(dir: &Path, id: &str) -> String {
        let path = dir.join(".tickets").join(format!("{id}.md"));
        let content = fs::read_to_string(path).unwrap();
        content
            .lines()
            .find(|l| l.starts_with("status: "))
            .unwrap()
            .trim_start_matches("status: ")
            .to_string()
    }

    // ------------------------------------------------------------------
    // start / close / reopen
    // ------------------------------------------------------------------

    #[test]
    fn start_sets_in_progress() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "test-0001", "open", "");
        set_status_impl(Some(tmp.path()), "test-0001", Status::InProgress).unwrap();
        assert_eq!(read_status(tmp.path(), "test-0001"), "in_progress");
    }

    #[test]
    fn close_sets_closed() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "test-0001", "open", "");
        set_status_impl(Some(tmp.path()), "test-0001", Status::Closed).unwrap();
        assert_eq!(read_status(tmp.path(), "test-0001"), "closed");
    }

    #[test]
    fn reopen_sets_open() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "test-0001", "closed", "");
        set_status_impl(Some(tmp.path()), "test-0001", Status::Open).unwrap();
        assert_eq!(read_status(tmp.path(), "test-0001"), "open");
    }

    // ------------------------------------------------------------------
    // status — explicit values
    // ------------------------------------------------------------------

    #[test]
    fn status_explicit_in_progress() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "test-0001", "open", "");
        let s = "in_progress".parse::<Status>().unwrap();
        set_status_impl(Some(tmp.path()), "test-0001", s).unwrap();
        assert_eq!(read_status(tmp.path(), "test-0001"), "in_progress");
    }

    #[test]
    fn status_explicit_closed() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "test-0001", "open", "");
        let s = "closed".parse::<Status>().unwrap();
        set_status_impl(Some(tmp.path()), "test-0001", s).unwrap();
        assert_eq!(read_status(tmp.path(), "test-0001"), "closed");
    }

    #[test]
    fn status_explicit_open() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "test-0001", "closed", "");
        let s = "open".parse::<Status>().unwrap();
        set_status_impl(Some(tmp.path()), "test-0001", s).unwrap();
        assert_eq!(read_status(tmp.path(), "test-0001"), "open");
    }

    // ------------------------------------------------------------------
    // Error cases
    // ------------------------------------------------------------------

    #[test]
    fn invalid_status_value() {
        let err = "invalid".parse::<Status>().unwrap_err();
        match err {
            Error::InvalidStatus { ref value, .. } => {
                assert_eq!(value, "invalid");
                // The Display impl should name the value and list valid options.
                let msg = format!("{err}");
                assert!(msg.contains("invalid"), "message missing the bad value");
                assert!(
                    msg.contains("open") && msg.contains("in_progress") && msg.contains("closed"),
                    "message missing valid options: {msg}"
                );
            }
            other => panic!("expected InvalidStatus, got {other:?}"),
        }
    }

    #[test]
    fn non_existent_ticket() {
        let tmp = tempdir().unwrap();
        // Create a store dir but no ticket files.
        fs::create_dir_all(tmp.path().join(".tickets")).unwrap();
        let err = set_status_impl(Some(tmp.path()), "ghost-0000", Status::Open).unwrap_err();
        assert!(
            matches!(err, Error::TicketNotFound { .. }),
            "expected TicketNotFound, got {err:?}"
        );
    }

    // ------------------------------------------------------------------
    // File content preserved
    // ------------------------------------------------------------------

    #[test]
    fn file_content_preserved() {
        let tmp = tempdir().unwrap();
        let extra_body = "\nSome description text.\n\n## Notes\n\nImportant notes here.\n";
        make_store_with_ticket(tmp.path(), "test-0001", "open", extra_body);

        // Read the original file to capture the body portion.
        let before = fs::read_to_string(tmp.path().join(".tickets").join("test-0001.md")).unwrap();

        set_status_impl(Some(tmp.path()), "test-0001", Status::Closed).unwrap();

        let after = fs::read_to_string(tmp.path().join(".tickets").join("test-0001.md")).unwrap();

        // Only the status line should have changed.
        assert_eq!(
            before.replace("status: open", "status: closed"),
            after,
            "file content differed beyond the status line"
        );

        // Body text is intact.
        assert!(after.contains("## Notes"), "## Notes section was lost");
        assert!(
            after.contains("Important notes here."),
            "body text was lost"
        );
    }

    // ------------------------------------------------------------------
    // Partial ID resolution
    // ------------------------------------------------------------------

    #[test]
    fn partial_id_resolution() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "test-9999", "open", "");
        // Resolve by suffix only.
        set_status_impl(Some(tmp.path()), "9999", Status::InProgress).unwrap();
        assert_eq!(read_status(tmp.path(), "test-9999"), "in_progress");
    }

    // ------------------------------------------------------------------
    // Output message
    // ------------------------------------------------------------------

    #[test]
    fn output_message() {
        // set_status_impl returns the exact string passed to println! by each
        // public function, so asserting it here verifies the printed output.
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "test-0001", "open", "");
        let msg = set_status_impl(Some(tmp.path()), "test-0001", Status::Closed).unwrap();
        assert_eq!(msg, "Updated test-0001 -> closed");
    }
}
