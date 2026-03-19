// .tickets/ directory operations: file read/write/list, partial ID resolution,
// and directory walking up parent paths.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::suggest;
use crate::ticket::Ticket;

// ---------------------------------------------------------------------------
// TicketStore
// ---------------------------------------------------------------------------

/// A handle to a resolved `.tickets/` directory.
///
/// All operations on ticket files go through this struct. Construct one with
/// [`TicketStore::find`] (which walks parent directories) or
/// [`TicketStore::ensure`] (which also creates the directory if absent).
#[derive(Debug)]
pub struct TicketStore {
    dir: PathBuf,
}

impl TicketStore {
    // -----------------------------------------------------------------------
    // Constructors
    // -----------------------------------------------------------------------

    /// Construct a store backed by an explicit directory path.
    ///
    /// Intended for tests and other contexts where the caller has already
    /// resolved the path and does not want the normal upward-walk logic.
    #[cfg(test)]
    pub(crate) fn from_dir(dir: PathBuf) -> Self {
        TicketStore { dir }
    }

    /// Resolve the `.tickets/` directory, optionally accepting an explicit
    /// override path (for testing or the `TICKETS_DIR` env var).
    ///
    /// If `override_dir` is `Some`, it is used directly without walking.
    /// Otherwise the search starts at `start_dir` (defaults to cwd) and walks
    /// ancestor directories until `.tickets/` is found.
    fn find_impl(start_dir: Option<&Path>, override_dir: Option<PathBuf>) -> Result<Self> {
        if let Some(dir) = override_dir {
            if !dir.is_dir() {
                return Err(Error::TicketDirNotFound { dir });
            }
            return Ok(TicketStore { dir });
        }

        let cwd;
        let start = match start_dir {
            Some(p) => p,
            None => {
                cwd = std::env::current_dir()?;
                cwd.as_path()
            }
        };

        let mut current = start;
        loop {
            let candidate = current.join(".tickets");
            if candidate.is_dir() {
                return Ok(TicketStore { dir: candidate });
            }
            match current.parent() {
                Some(parent) => current = parent,
                None => return Err(Error::TicketsNotFound),
            }
        }
    }

    /// Find the `.tickets/` directory by walking parent directories from
    /// `start_dir` (or cwd).
    ///
    /// The override directory is resolved from [`crate::config::global`] which
    /// honours `TICKET_DIR` (and the legacy `TICKETS_DIR`) env vars as well as
    /// the `ticket_dir` key in `.tickets.toml`.
    pub fn find(start_dir: Option<&Path>) -> Result<Self> {
        let override_dir = crate::config::global().ticket_dir.clone();
        Self::find_impl(start_dir, override_dir)
    }

    /// Like [`TicketStore::find`], but creates the ticket directory when it
    /// does not yet exist — subject to safety constraints.
    ///
    /// * When the directory was **explicitly configured** (`TICKET_DIR` /
    ///   `.tickets.toml`) and the configured path is missing, only the **final
    ///   path segment** is created.  If its parent does not exist an
    ///   [`Error::TicketDirParentNotFound`] is returned — the tool will never
    ///   silently create deep directory trees for a user-supplied path.
    ///
    /// * In the **default** (no explicit override) case the directory is the
    ///   literal `.tickets/` entry inside an existing `start_dir` / cwd, so
    ///   creating it with [`std::fs::create_dir`] is always safe.
    ///
    /// In both cases a note is printed to stderr when the directory is newly
    /// created.
    pub fn ensure(start_dir: Option<&Path>) -> Result<Self> {
        let override_dir = crate::config::global().ticket_dir.clone();
        Self::ensure_impl(start_dir, override_dir)
    }

    /// Core logic for [`TicketStore::ensure`], accepting an explicit override
    /// so tests can exercise it without touching process-global config state.
    fn ensure_impl(start_dir: Option<&Path>, override_dir: Option<PathBuf>) -> Result<Self> {
        match Self::find_impl(start_dir, override_dir.clone()) {
            Ok(store) => Ok(store),

            // Configured path missing — only create the final segment.
            Err(Error::TicketDirNotFound { dir }) => {
                let parent = dir
                    .parent()
                    .ok_or_else(|| Error::TicketDirParentNotFound { dir: dir.clone() })?;
                if !parent.is_dir() {
                    return Err(Error::TicketDirParentNotFound { dir });
                }
                std::fs::create_dir(&dir)?;
                eprintln!("Created ticket directory: {}", dir.display());
                Ok(TicketStore { dir })
            }

            // Default walk found nothing — create .tickets/ in start_dir/cwd.
            Err(Error::TicketsNotFound) if override_dir.is_none() => {
                let cwd;
                let base = match start_dir {
                    Some(p) => p,
                    None => {
                        cwd = std::env::current_dir()?;
                        cwd.as_path()
                    }
                };
                let dir = base.join(".tickets");
                std::fs::create_dir(&dir)?;
                eprintln!("Created ticket directory: {}", dir.display());
                Ok(TicketStore { dir })
            }

            Err(e) => Err(e),
        }
    }

    // -----------------------------------------------------------------------
    // Directory helpers
    // -----------------------------------------------------------------------

    /// Return the resolved path to the `.tickets/` directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Create the `.tickets/` directory if it does not already exist.
    pub fn ensure_dir(&self) -> Result<()> {
        if !self.dir.is_dir() {
            std::fs::create_dir(&self.dir)?;
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // File operations
    // -----------------------------------------------------------------------

    /// Read and parse the ticket with the given full ID.
    pub fn read_ticket(&self, id: &str) -> Result<Ticket> {
        let path = self.dir.join(format!("{id}.md"));
        let content = std::fs::read_to_string(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::TicketNotFound {
                    id: id.to_string(),
                    suggestions: vec![],
                }
            } else {
                Error::Io(e)
            }
        })?;
        Ticket::read_from_str(&content)
    }

    /// Write a ticket to disk, overwriting any existing file.
    pub fn write_ticket(&self, ticket: &Ticket) -> Result<()> {
        let path = self.dir.join(format!("{}.md", ticket.id));
        std::fs::write(&path, ticket.write_to_string())?;
        Ok(())
    }

    /// Read all `.md` files in the tickets directory and return parsed tickets.
    /// Files that fail to parse are silently skipped.
    pub fn list_tickets(&self) -> Vec<Ticket> {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        entries
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension()?.to_str()? != "md" {
                    return None;
                }
                let content = std::fs::read_to_string(&path).ok()?;
                Ticket::read_from_str(&content).ok()
            })
            .collect()
    }

    /// Return ticket file paths sorted by modification time (most recent first).
    ///
    /// Files that cannot be stat'd are placed at the end of the list.
    pub fn paths_by_mtime(&self) -> Vec<PathBuf> {
        use std::time::SystemTime;

        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };

        let mut paths_with_mtime: Vec<(PathBuf, SystemTime)> = entries
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension()?.to_str()? != "md" {
                    return None;
                }
                let mtime = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                Some((path, mtime))
            })
            .collect();

        // Sort most-recent first.
        paths_with_mtime.sort_by(|a, b| b.1.cmp(&a.1));
        paths_with_mtime.into_iter().map(|(p, _)| p).collect()
    }

    // -----------------------------------------------------------------------
    // ID resolution
    // -----------------------------------------------------------------------

    /// Collect the ID stems (file names without `.md`) from the tickets
    /// directory.
    fn ticket_ids(&self) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        entries
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension()?.to_str()? != "md" {
                    return None;
                }
                Some(path.file_stem()?.to_string_lossy().into_owned())
            })
            .collect()
    }

    /// Resolve a partial (or full) ticket ID to the path of its `.md` file.
    ///
    /// Resolution rules (in priority order):
    /// 1. Exact match — returned immediately, even if other IDs contain it as a
    ///    substring.
    /// 2. Substring match — `partial` is contained within the candidate ID.
    ///    If exactly one candidate matches, its path is returned.
    ///    If multiple match, an [`Error::AmbiguousId`] is returned.
    ///    If none match, an [`Error::TicketNotFound`] is returned.
    pub fn resolve_id(&self, partial: &str) -> Result<PathBuf> {
        let ids = self.ticket_ids();

        // 1. Exact match takes precedence.
        if ids.iter().any(|id| id == partial) {
            return Ok(self.dir.join(format!("{partial}.md")));
        }

        // 2. Substring match.
        let mut matches: Vec<String> = ids.into_iter().filter(|id| id.contains(partial)).collect();

        match matches.len() {
            0 => {
                let all_tickets = self.list_tickets();
                let suggestions = suggest::suggest_tickets(partial, &all_tickets, 3);
                Err(Error::TicketNotFound {
                    id: partial.to_string(),
                    suggestions,
                })
            }
            1 => Ok(self.dir.join(format!("{}.md", matches.remove(0)))),
            _ => {
                matches.sort();
                Err(Error::AmbiguousId {
                    partial: partial.to_string(),
                    candidates: matches,
                })
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    use crate::ticket::{Status, Ticket, TicketType};

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Create a minimal valid ticket content string for the given id.
    fn ticket_content(id: &str) -> String {
        format!(
            "---\nid: {id}\nstatus: open\ndeps: []\nlinks: []\ncreated: 2026-01-01T00:00:00Z\ntype: task\npriority: 2\n---\n# Test ticket {id}\n"
        )
    }

    /// Write a `.md` file with the given id into the `.tickets/` dir inside `root`.
    fn write_ticket_file(tickets_dir: &Path, id: &str) {
        std::fs::write(tickets_dir.join(format!("{id}.md")), ticket_content(id)).unwrap();
    }

    /// Build a store backed by a fresh `.tickets/` dir inside `root`.
    fn make_store(root: &TempDir) -> TicketStore {
        let dir = root.path().join(".tickets");
        std::fs::create_dir_all(&dir).unwrap();
        TicketStore { dir }
    }

    // -----------------------------------------------------------------------
    // find_tickets_dir — current directory
    // -----------------------------------------------------------------------

    #[test]
    fn find_tickets_dir_current_directory() {
        let root = tempfile::tempdir().unwrap();
        let tickets_dir = root.path().join(".tickets");
        std::fs::create_dir_all(&tickets_dir).unwrap();

        let store = TicketStore::find_impl(Some(root.path()), None).unwrap();
        assert_eq!(store.dir, tickets_dir);
    }

    // -----------------------------------------------------------------------
    // find_tickets_dir — parent directory
    // -----------------------------------------------------------------------

    #[test]
    fn find_tickets_dir_parent_directory() {
        let root = tempfile::tempdir().unwrap();
        let tickets_dir = root.path().join(".tickets");
        std::fs::create_dir_all(&tickets_dir).unwrap();

        let nested = root.path().join("src").join("components");
        std::fs::create_dir_all(&nested).unwrap();

        let store = TicketStore::find_impl(Some(&nested), None).unwrap();
        assert_eq!(store.dir, tickets_dir);
    }

    // -----------------------------------------------------------------------
    // find_tickets_dir — grandparent directory
    // -----------------------------------------------------------------------

    #[test]
    fn find_tickets_dir_grandparent_directory() {
        let root = tempfile::tempdir().unwrap();
        let tickets_dir = root.path().join(".tickets");
        std::fs::create_dir_all(&tickets_dir).unwrap();

        let nested = root.path().join("src").join("components").join("ui");
        std::fs::create_dir_all(&nested).unwrap();

        let store = TicketStore::find_impl(Some(&nested), None).unwrap();
        assert_eq!(store.dir, tickets_dir);
    }

    // -----------------------------------------------------------------------
    // find_tickets_dir — TICKETS_DIR env override
    // -----------------------------------------------------------------------

    #[test]
    fn find_tickets_dir_env_override() {
        let root = tempfile::tempdir().unwrap();
        // No .tickets/ in root — if walking were used it would fail.
        let override_dir = root.path().join("custom-tickets");
        std::fs::create_dir_all(&override_dir).unwrap();

        // Pass the override explicitly to avoid mutating global env state.
        let store = TicketStore::find_impl(Some(root.path()), Some(override_dir.clone())).unwrap();
        assert_eq!(store.dir, override_dir);
    }

    // -----------------------------------------------------------------------
    // find_tickets_dir — not found
    // -----------------------------------------------------------------------

    #[test]
    fn find_tickets_dir_not_found() {
        let root = tempfile::tempdir().unwrap();
        // No .tickets/ anywhere under root.
        let err = TicketStore::find_impl(Some(root.path()), None).unwrap_err();
        assert!(
            matches!(err, Error::TicketsNotFound),
            "expected TicketsNotFound, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // find_impl — override path does not exist → TicketDirNotFound
    // -----------------------------------------------------------------------

    #[test]
    fn find_impl_override_nonexistent_yields_ticket_dir_not_found() {
        let root = tempfile::tempdir().unwrap();
        let nonexistent = root.path().join("does-not-exist");

        let err = TicketStore::find_impl(Some(root.path()), Some(nonexistent.clone())).unwrap_err();
        assert!(
            matches!(err, Error::TicketDirNotFound { ref dir } if dir == &nonexistent),
            "expected TicketDirNotFound, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // ensure_impl — configured override: creates last segment when parent exists
    // -----------------------------------------------------------------------

    #[test]
    fn ensure_impl_override_creates_dir_when_parent_exists() {
        let root = tempfile::tempdir().unwrap();
        // The configured path does not exist yet, but its parent (root) does.
        let new_dir = root.path().join("my-tickets");
        assert!(!new_dir.exists(), "precondition: dir should not exist yet");

        let store = TicketStore::ensure_impl(Some(root.path()), Some(new_dir.clone())).unwrap();

        assert!(
            new_dir.is_dir(),
            "ensure_impl should have created my-tickets/"
        );
        assert_eq!(store.dir(), new_dir);
    }

    // -----------------------------------------------------------------------
    // ensure_impl — configured override: fails when parent does not exist
    // -----------------------------------------------------------------------

    #[test]
    fn ensure_impl_override_fails_when_parent_does_not_exist() {
        let root = tempfile::tempdir().unwrap();
        // Neither "missing-parent" nor "my-tickets" exist.
        let new_dir = root.path().join("missing-parent").join("my-tickets");
        assert!(
            !new_dir.parent().unwrap().is_dir(),
            "precondition: parent must not exist"
        );

        let err = TicketStore::ensure_impl(Some(root.path()), Some(new_dir.clone())).unwrap_err();
        assert!(
            matches!(err, Error::TicketDirParentNotFound { ref dir } if dir == &new_dir),
            "expected TicketDirParentNotFound, got {err:?}"
        );
        assert!(
            !new_dir.exists(),
            "ensure_impl must not have created any directories"
        );
    }

    // -----------------------------------------------------------------------
    // ensure_impl — no override: creates .tickets/ in start_dir
    // -----------------------------------------------------------------------

    #[test]
    fn ensure_impl_default_creates_tickets_dir_in_start_dir() {
        let root = tempfile::tempdir().unwrap();
        // No .tickets/ yet and no override — should create root/.tickets/
        let expected = root.path().join(".tickets");
        assert!(
            !expected.exists(),
            "precondition: .tickets/ should not exist yet"
        );

        let store = TicketStore::ensure_impl(Some(root.path()), None).unwrap();

        assert!(
            expected.is_dir(),
            "ensure_impl should have created .tickets/"
        );
        assert_eq!(store.dir(), expected);
    }

    // -----------------------------------------------------------------------
    // ensure_impl — no override: does not create parent directories
    // -----------------------------------------------------------------------

    #[test]
    fn ensure_impl_default_does_not_use_create_dir_all() {
        // Verify that ensure_impl with no override uses create_dir (single
        // level) rather than create_dir_all.  If the start_dir itself doesn't
        // exist the call must fail rather than creating a whole tree.
        let root = tempfile::tempdir().unwrap();
        let nonexistent_start = root.path().join("no-such-dir");
        // start_dir doesn't exist — create_dir on start_dir/.tickets should fail.
        let result = TicketStore::ensure_impl(Some(&nonexistent_start), None);
        assert!(
            result.is_err(),
            "ensure_impl should not create intermediate directories"
        );
    }

    // -----------------------------------------------------------------------
    // error display — TicketDirNotFound
    // -----------------------------------------------------------------------

    #[test]
    fn ticket_dir_not_found_display_contains_path() {
        let err = Error::TicketDirNotFound {
            dir: std::path::PathBuf::from("/some/missing/path"),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("/some/missing/path"),
            "expected path in message, got: {msg}"
        );
        assert!(
            msg.contains("TICKET_DIR"),
            "expected 'TICKET_DIR' in message, got: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // error display — TicketDirParentNotFound
    // -----------------------------------------------------------------------

    #[test]
    fn ticket_dir_parent_not_found_display_contains_path() {
        let err = Error::TicketDirParentNotFound {
            dir: std::path::PathBuf::from("/nonexistent/parent/dir"),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("/nonexistent/parent/dir"),
            "expected path in message, got: {msg}"
        );
        assert!(
            msg.contains("TICKET_DIR"),
            "expected 'TICKET_DIR' in message, got: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // resolve_id — exact match
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_id_exact_match() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);
        write_ticket_file(store.dir(), "abc-1234");

        let resolved = store.resolve_id("abc-1234").unwrap();
        assert_eq!(resolved, store.dir().join("abc-1234.md"));
    }

    // -----------------------------------------------------------------------
    // resolve_id — prefix match
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_id_prefix_match() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);
        write_ticket_file(store.dir(), "abc-1234");

        let resolved = store.resolve_id("abc").unwrap();
        assert_eq!(resolved, store.dir().join("abc-1234.md"));
    }

    // -----------------------------------------------------------------------
    // resolve_id — suffix match
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_id_suffix_match() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);
        write_ticket_file(store.dir(), "abc-1234");

        let resolved = store.resolve_id("1234").unwrap();
        assert_eq!(resolved, store.dir().join("abc-1234.md"));
    }

    // -----------------------------------------------------------------------
    // resolve_id — substring match
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_id_substring_match() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);
        write_ticket_file(store.dir(), "abc-1234");

        let resolved = store.resolve_id("c-12").unwrap();
        assert_eq!(resolved, store.dir().join("abc-1234.md"));
    }

    // -----------------------------------------------------------------------
    // resolve_id — exact takes precedence over partial
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_id_exact_takes_precedence() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);
        write_ticket_file(store.dir(), "abc");
        write_ticket_file(store.dir(), "abc-1234");

        // "abc" is an exact match for "abc.md", so it must win over the
        // substring match on "abc-1234".
        let resolved = store.resolve_id("abc").unwrap();
        assert_eq!(resolved, store.dir().join("abc.md"));
    }

    // -----------------------------------------------------------------------
    // resolve_id — ambiguous error
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_id_ambiguous_error() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);
        write_ticket_file(store.dir(), "abc-1234");
        write_ticket_file(store.dir(), "abc-5678");

        let err = store.resolve_id("abc").unwrap_err();
        match err {
            Error::AmbiguousId {
                partial,
                candidates,
            } => {
                assert_eq!(partial, "abc");
                assert!(
                    candidates.contains(&"abc-1234".to_string()),
                    "expected abc-1234 in candidates"
                );
                assert!(
                    candidates.contains(&"abc-5678".to_string()),
                    "expected abc-5678 in candidates"
                );
            }
            other => panic!("expected AmbiguousId, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // resolve_id — not found error
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_id_not_found_error() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);

        let err = store.resolve_id("nonexistent").unwrap_err();
        assert!(
            matches!(err, Error::TicketNotFound { .. }),
            "expected TicketNotFound, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // read_ticket / write_ticket — round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn read_write_round_trip() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);

        let original = Ticket {
            id: "tr-test".to_string(),
            status: Status::Open,
            deps: vec!["tr-dep1".to_string()],
            links: vec![],
            created: "2026-03-08T06:29:51Z"
                .parse::<chrono::DateTime<Utc>>()
                .unwrap(),
            ticket_type: TicketType::Task,
            priority: 2,
            assignee: Some("Alice".to_string()),
            external_ref: None,
            parent: None,
            tags: Some(vec!["phase-1".to_string()]),
            title: "Test Ticket".to_string(),
            body: "# Test Ticket\n\nSome description.\n".to_string(),
        };

        store.write_ticket(&original).unwrap();
        let read_back = store.read_ticket("tr-test").unwrap();

        assert_eq!(read_back.id, original.id);
        assert_eq!(read_back.status, original.status);
        assert_eq!(read_back.deps, original.deps);
        assert_eq!(read_back.links, original.links);
        assert_eq!(read_back.created, original.created);
        assert_eq!(read_back.ticket_type, original.ticket_type);
        assert_eq!(read_back.priority, original.priority);
        assert_eq!(read_back.assignee, original.assignee);
        assert_eq!(read_back.external_ref, original.external_ref);
        assert_eq!(read_back.parent, original.parent);
        assert_eq!(read_back.tags, original.tags);
        assert_eq!(read_back.title, original.title);
        assert_eq!(read_back.body, original.body);
    }

    // -----------------------------------------------------------------------
    // read_ticket — missing file yields TicketNotFound, not Io
    // -----------------------------------------------------------------------

    #[test]
    fn read_ticket_missing_file_yields_ticket_not_found() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);

        let err = store.read_ticket("no-such-id").unwrap_err();
        assert!(
            matches!(err, Error::TicketNotFound { .. }),
            "expected TicketNotFound for missing file, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // list_tickets — returns all tickets
    // -----------------------------------------------------------------------

    #[test]
    fn list_tickets_returns_all() {
        let root = tempfile::tempdir().unwrap();
        let store = make_store(&root);
        write_ticket_file(store.dir(), "tr-aaa1");
        write_ticket_file(store.dir(), "tr-bbb2");
        write_ticket_file(store.dir(), "tr-ccc3");

        let mut tickets = store.list_tickets();
        tickets.sort_by(|a, b| a.id.cmp(&b.id));
        let ids: Vec<&str> = tickets.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["tr-aaa1", "tr-bbb2", "tr-ccc3"]);
    }

    // -----------------------------------------------------------------------
    // ensure_dir — creates directory
    // -----------------------------------------------------------------------

    #[test]
    fn ensure_dir_creates_directory() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join(".tickets");
        assert!(!dir.exists(), "expected .tickets/ to not exist yet");

        let store = TicketStore { dir: dir.clone() };
        store.ensure_dir().unwrap();

        assert!(dir.is_dir(), "expected .tickets/ to exist after ensure_dir");
    }
}
