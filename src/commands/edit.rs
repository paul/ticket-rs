// Edit command: open a ticket file in $EDITOR (TTY) or print its path (non-TTY).
//
// When stdout is a terminal the command launches $EDITOR (falling back to vi)
// and waits for it to exit.  When stdout is not a terminal — the case for
// scripts, agents, and the BDD integration tests — the command prints the
// resolved absolute path so the caller can open the file however it likes.

use std::io::IsTerminal as _;
use std::path::Path;
use std::process::Command;

use crate::error::Result;
use crate::store::TicketStore;

fn edit_impl(start_dir: Option<&Path>, partial_id: &str) -> Result<String> {
    let store = TicketStore::find(start_dir)?;

    let path = store.resolve_id(partial_id)?;

    if std::io::stdout().is_terminal() {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
        let exit_status = Command::new(&editor).arg(&path).status()?;
        if !exit_status.success() {
            return Err(crate::error::Error::EditorError {
                editor,
                code: exit_status.code(),
            });
        }
        Ok(format!("Opened {} in {editor}", path.display()))
    } else {
        Ok(format!("Edit ticket file: {}", path.display()))
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn edit(id: &str) -> Result<()> {
    let msg = edit_impl(None, id)?;
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

    fn make_store_with_ticket(dir: &Path, id: &str) {
        let tickets_dir = dir.join(".tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        let content = format!(
            "---\nid: {id}\nstatus: open\ndeps: []\nlinks: []\ncreated: 2026-01-01T00:00:00Z\ntype: task\npriority: 2\n---\n# Editable ticket\n"
        );
        fs::write(tickets_dir.join(format!("{id}.md")), content).unwrap();
    }

    // edit_impl is always running with stdout redirected in tests (non-TTY),
    // so we exercise the non-TTY branch here.

    #[test]
    fn non_tty_returns_file_path() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "edit-0001");

        let msg = edit_impl(Some(tmp.path()), "edit-0001").unwrap();

        assert!(
            msg.contains("Edit ticket file:"),
            "expected 'Edit ticket file:' prefix, got: {msg}"
        );
        assert!(
            msg.contains("edit-0001.md"),
            "expected file name in output, got: {msg}"
        );
    }

    #[test]
    fn non_tty_partial_id_resolution() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "edit-0001");

        let msg = edit_impl(Some(tmp.path()), "0001").unwrap();

        assert!(
            msg.contains("edit-0001.md"),
            "expected resolved file name in output, got: {msg}"
        );
    }

    #[test]
    fn non_existent_ticket_returns_error() {
        let tmp = tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".tickets")).unwrap();

        let err = edit_impl(Some(tmp.path()), "nonexistent").unwrap_err();
        assert!(
            matches!(err, Error::TicketNotFound { .. }),
            "expected TicketNotFound, got {err:?}"
        );
    }
}
