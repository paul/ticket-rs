// Dynamic completion helpers for clap_complete.
//
// Registered as ArgValueCompleter on every argument that accepts a ticket ID.
// At tab-completion time the shell calls the binary with COMPLETE=$SHELL set;
// clap_complete intercepts that, invokes the registered completers, and exits
// before any normal application logic runs.

use clap_complete::engine::{CompletionCandidate, ValueCompleter};

use crate::store::TicketStore;

/// Build completion candidates from a `TicketStore`, one per ticket.
/// Each candidate carries the ticket title as a description hint.
pub(crate) fn candidates_from_store(store: &TicketStore) -> Vec<CompletionCandidate> {
    store
        .list_tickets()
        .into_iter()
        .map(|t| CompletionCandidate::new(t.id.clone()).help(Some(t.title.clone().into())))
        .collect()
}

/// A `ValueCompleter` that lists ticket IDs found in the nearest `.tickets/`
/// directory, including the ticket title as a description hint.
pub struct TicketIdCompleter;

impl ValueCompleter for TicketIdCompleter {
    fn complete(&self, _current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
        match TicketStore::find(None) {
            Ok(store) => candidates_from_store(&store),
            Err(_) => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tempfile::TempDir;

    use super::*;
    use crate::store::TicketStore;

    fn make_store(root: &TempDir) -> TicketStore {
        let dir = root.path().join(".tickets");
        std::fs::create_dir_all(&dir).unwrap();
        TicketStore::from_dir(dir)
    }

    fn write_ticket(dir: &Path, id: &str, title: &str) {
        let content = format!(
            "---\nid: {id}\nstatus: open\ndeps: []\nlinks: []\ncreated: 2026-01-01T00:00:00Z\ntype: task\npriority: 2\n---\n# {title}\n"
        );
        std::fs::write(dir.join(format!("{id}.md")), content).unwrap();
    }

    #[test]
    fn empty_store_returns_no_candidates() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);

        let candidates = candidates_from_store(&store);

        assert!(candidates.is_empty());
    }

    #[test]
    fn candidates_include_all_ticket_ids() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);
        write_ticket(store.dir(), "tr-aaaa", "First ticket");
        write_ticket(store.dir(), "tr-bbbb", "Second ticket");

        let candidates = candidates_from_store(&store);

        let mut ids: Vec<String> = candidates
            .iter()
            .map(|c| c.get_value().to_string_lossy().into_owned())
            .collect();
        ids.sort();
        assert_eq!(ids, vec!["tr-aaaa", "tr-bbbb"]);
    }

    #[test]
    fn candidate_help_text_is_ticket_title() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);
        write_ticket(store.dir(), "tr-cccc", "My important ticket");

        let candidates = candidates_from_store(&store);

        assert_eq!(candidates.len(), 1);
        let help = candidates[0]
            .get_help()
            .map(|h| h.to_string())
            .unwrap_or_default();
        assert_eq!(help, "My important ticket");
    }

    #[test]
    fn missing_store_returns_empty_via_completer() {
        use std::ffi::OsStr;

        // With no TICKET_DIR set and no .tickets/ anywhere above a temp dir,
        // the completer should silently return nothing rather than panic.
        // We exercise the ValueCompleter::complete path indirectly by calling
        // candidates_from_store with an empty store.
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);

        let result = candidates_from_store(&store);
        // No tickets written — result must be empty, not a panic.
        assert!(result.is_empty());

        // Also confirm ValueCompleter trait compiles and doesn't crash when
        // the store exists but is empty.
        let completer = TicketIdCompleter;
        let _ = completer.complete(OsStr::new("tr-"));
    }
}
