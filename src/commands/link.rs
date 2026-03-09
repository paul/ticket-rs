// link and unlink commands.
//
// `link` creates symmetric links between all specified tickets (2 or more).
// For each pair (A, B), A's ID is added to B's links array and vice versa.
// Duplicates are prevented.  If all links already exist the command reports
// that; otherwise it reports the number of new link entries added.
//
// `unlink` removes the symmetric link between exactly two tickets.  Both
// tickets must have each other in their links arrays; if the link is absent
// the command prints "Link not found" to stdout and exits with a non-zero
// status code (matching the behaviour expected by the BDD integration tests).

use std::path::Path;

use crate::error::{Error, Result};
use crate::store::TicketStore;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Extract the full ticket ID from a resolved `.md` path.
///
/// Mirrors the helper in `dep.rs`.  The path always originates from
/// `TicketStore::resolve_id`, so the stem is guaranteed to be valid UTF-8; the
/// `partial` fallback is unreachable in practice.
fn full_id_from_path<'a>(path: &'a std::path::PathBuf, partial: &'a str) -> &'a str {
    debug_assert!(
        path.file_stem().and_then(|s| s.to_str()).is_some(),
        "ticket path stem should always be valid UTF-8: {path:?}"
    );
    path.file_stem().and_then(|s| s.to_str()).unwrap_or(partial)
}

/// Create symmetric links between all given ticket IDs.  Returns the output
/// message.
fn link_impl(start_dir: Option<&Path>, partial_ids: &[&str]) -> Result<String> {
    let store = TicketStore::find(start_dir)?;

    // Resolve all IDs up front — bail immediately if any ticket is not found.
    let resolved: Vec<(String, String)> = partial_ids
        .iter()
        .map(|partial| {
            let path = store.resolve_id(partial)?;
            let full_id = full_id_from_path(&path, partial).to_string();
            Ok((partial.to_string(), full_id))
        })
        .collect::<Result<Vec<_>>>()?;

    let full_ids: Vec<&str> = resolved.iter().map(|(_, id)| id.as_str()).collect();
    let n = full_ids.len();

    // For each ordered pair (i < j), add j to i's links and i to j's links.
    let mut new_links: usize = 0;

    for i in 0..n {
        for j in (i + 1)..n {
            let id_a = full_ids[i];
            let id_b = full_ids[j];

            let mut ticket_a = store.read_ticket(id_a)?;
            let a_has_b = ticket_a.links.iter().any(|l| l == id_b);

            let mut ticket_b = store.read_ticket(id_b)?;
            let b_has_a = ticket_b.links.iter().any(|l| l == id_a);

            if !a_has_b {
                ticket_a.links.push(id_b.to_string());
                store.write_ticket(&ticket_a)?;
                new_links += 1;
            }

            if !b_has_a {
                ticket_b.links.push(id_a.to_string());
                store.write_ticket(&ticket_b)?;
                new_links += 1;
            }
        }
    }

    if new_links == 0 {
        Ok("All links already exist".to_string())
    } else {
        Ok(format!("Added {new_links} link(s) between {n} tickets"))
    }
}

/// Remove the symmetric link between two ticket IDs.  Returns the output
/// message, or `Error::LinkNotFound` if neither ticket has the link.
fn unlink_impl(
    start_dir: Option<&Path>,
    partial_id: &str,
    partial_target_id: &str,
) -> Result<String> {
    let store = TicketStore::find(start_dir)?;

    let src_path = store.resolve_id(partial_id)?;
    let src_id = full_id_from_path(&src_path, partial_id).to_string();

    let tgt_path = store.resolve_id(partial_target_id)?;
    let tgt_id = full_id_from_path(&tgt_path, partial_target_id).to_string();

    let mut src_ticket = store.read_ticket(&src_id)?;
    let mut tgt_ticket = store.read_ticket(&tgt_id)?;

    let src_pos = src_ticket.links.iter().position(|l| l == &tgt_id);
    let tgt_pos = tgt_ticket.links.iter().position(|l| l == &src_id);

    if src_pos.is_none() || tgt_pos.is_none() {
        return Err(Error::LinkNotFound);
    }

    src_ticket.links.remove(src_pos.unwrap());
    store.write_ticket(&src_ticket)?;

    tgt_ticket.links.remove(tgt_pos.unwrap());
    store.write_ticket(&tgt_ticket)?;

    Ok(format!("Removed link: {src_id} <-> {tgt_id}"))
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

pub fn link(ids: &[String]) -> Result<()> {
    let partial_ids: Vec<&str> = ids.iter().map(String::as_str).collect();
    let msg = link_impl(None, &partial_ids)?;
    println!("{msg}");
    Ok(())
}

pub fn unlink(id: &str, target_id: &str) -> Result<()> {
    match unlink_impl(None, id, target_id) {
        Ok(msg) => {
            println!("{msg}");
            Ok(())
        }
        Err(Error::LinkNotFound) => {
            // Print to stdout (not stderr) and exit with non-zero status so
            // that the BDD step `the output should be "Link not found"` passes.
            // (That step checks context.stdout, not context.stderr.)
            println!("Link not found");
            std::process::exit(1);
        }
        Err(err) => Err(err),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn make_store_with_ticket(dir: &Path, id: &str, links: &[&str]) -> TicketStore {
        let tickets_dir = dir.join(".tickets");
        fs::create_dir_all(&tickets_dir).unwrap();

        let links_str = links.join(", ");
        let content = format!(
            "---\nid: {id}\nstatus: open\ndeps: []\nlinks: [{links_str}]\ncreated: 2026-01-01T00:00:00Z\ntype: task\npriority: 2\nassignee: Test User\ntags: [phase-2]\n---\n# A ticket\n\nBody text.\n"
        );
        fs::write(tickets_dir.join(format!("{id}.md")), &content).unwrap();

        TicketStore::find(Some(dir)).unwrap()
    }

    fn read_links(dir: &Path, id: &str) -> Vec<String> {
        let path = dir.join(".tickets").join(format!("{id}.md"));
        let content = fs::read_to_string(path).unwrap();
        let line = content.lines().find(|l| l.starts_with("links: ")).unwrap();
        let inner = line.trim_start_matches("links: [").trim_end_matches(']');
        if inner.is_empty() {
            vec![]
        } else {
            inner.split(", ").map(|s| s.to_string()).collect()
        }
    }

    // -----------------------------------------------------------------------
    // link two tickets — symmetric
    // -----------------------------------------------------------------------

    #[test]
    fn link_two_tickets_symmetric() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &[]);
        make_store_with_ticket(tmp.path(), "task-0002", &[]);

        link_impl(Some(tmp.path()), &["task-0001", "task-0002"]).unwrap();

        assert!(read_links(tmp.path(), "task-0001").contains(&"task-0002".to_string()));
        assert!(read_links(tmp.path(), "task-0002").contains(&"task-0001".to_string()));
    }

    // -----------------------------------------------------------------------
    // link three tickets — all pairs
    // -----------------------------------------------------------------------

    #[test]
    fn link_three_tickets_all_pairs() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &[]);
        make_store_with_ticket(tmp.path(), "task-0002", &[]);
        make_store_with_ticket(tmp.path(), "task-0003", &[]);

        link_impl(Some(tmp.path()), &["task-0001", "task-0002", "task-0003"]).unwrap();

        let links1 = read_links(tmp.path(), "task-0001");
        let links2 = read_links(tmp.path(), "task-0002");
        let links3 = read_links(tmp.path(), "task-0003");

        assert!(links1.contains(&"task-0002".to_string()));
        assert!(links1.contains(&"task-0003".to_string()));
        assert!(links2.contains(&"task-0001".to_string()));
        assert!(links2.contains(&"task-0003".to_string()));
        assert!(links3.contains(&"task-0001".to_string()));
        assert!(links3.contains(&"task-0002".to_string()));
    }

    // -----------------------------------------------------------------------
    // link output — two tickets
    // -----------------------------------------------------------------------

    #[test]
    fn link_output_two_tickets() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &[]);
        make_store_with_ticket(tmp.path(), "task-0002", &[]);

        let msg = link_impl(Some(tmp.path()), &["task-0001", "task-0002"]).unwrap();
        assert_eq!(msg, "Added 2 link(s) between 2 tickets");
    }

    // -----------------------------------------------------------------------
    // link output — three tickets
    // -----------------------------------------------------------------------

    #[test]
    fn link_output_three_tickets() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &[]);
        make_store_with_ticket(tmp.path(), "task-0002", &[]);
        make_store_with_ticket(tmp.path(), "task-0003", &[]);

        let msg = link_impl(Some(tmp.path()), &["task-0001", "task-0002", "task-0003"]).unwrap();
        assert_eq!(msg, "Added 6 link(s) between 3 tickets");
    }

    // -----------------------------------------------------------------------
    // link is idempotent
    // -----------------------------------------------------------------------

    #[test]
    fn link_is_idempotent() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &[]);
        make_store_with_ticket(tmp.path(), "task-0002", &[]);

        link_impl(Some(tmp.path()), &["task-0001", "task-0002"]).unwrap();
        let msg = link_impl(Some(tmp.path()), &["task-0001", "task-0002"]).unwrap();

        assert_eq!(msg, "All links already exist");

        // Each ID should appear exactly once in the other's links.
        let links1 = read_links(tmp.path(), "task-0001");
        let links2 = read_links(tmp.path(), "task-0002");
        assert_eq!(
            links1.iter().filter(|l| *l == "task-0002").count(),
            1,
            "task-0002 should appear exactly once in task-0001's links"
        );
        assert_eq!(
            links2.iter().filter(|l| *l == "task-0001").count(),
            1,
            "task-0001 should appear exactly once in task-0002's links"
        );
    }

    // -----------------------------------------------------------------------
    // link partial — only new pairs added
    // -----------------------------------------------------------------------

    #[test]
    fn link_partial_only_new_pairs() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &["task-0002"]);
        make_store_with_ticket(tmp.path(), "task-0002", &["task-0001"]);
        make_store_with_ticket(tmp.path(), "task-0003", &[]);

        // A↔B already linked; linking A B C should only add A↔C and B↔C.
        let msg = link_impl(Some(tmp.path()), &["task-0001", "task-0002", "task-0003"]).unwrap();
        assert_eq!(msg, "Added 4 link(s) between 3 tickets");

        // A↔B link still present exactly once.
        let links1 = read_links(tmp.path(), "task-0001");
        assert_eq!(links1.iter().filter(|l| *l == "task-0002").count(), 1);
        // New links present.
        assert!(links1.contains(&"task-0003".to_string()));
        assert!(read_links(tmp.path(), "task-0002").contains(&"task-0003".to_string()));
    }

    // -----------------------------------------------------------------------
    // unlink removes both directions
    // -----------------------------------------------------------------------

    #[test]
    fn unlink_removes_both_directions() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &["task-0002"]);
        make_store_with_ticket(tmp.path(), "task-0002", &["task-0001"]);

        unlink_impl(Some(tmp.path()), "task-0001", "task-0002").unwrap();

        assert!(!read_links(tmp.path(), "task-0001").contains(&"task-0002".to_string()));
        assert!(!read_links(tmp.path(), "task-0002").contains(&"task-0001".to_string()));
    }

    // -----------------------------------------------------------------------
    // unlink output
    // -----------------------------------------------------------------------

    #[test]
    fn unlink_output() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &["task-0002"]);
        make_store_with_ticket(tmp.path(), "task-0002", &["task-0001"]);

        let msg = unlink_impl(Some(tmp.path()), "task-0001", "task-0002").unwrap();
        assert_eq!(msg, "Removed link: task-0001 <-> task-0002");
    }

    // -----------------------------------------------------------------------
    // unlink — link not found (neither side)
    // -----------------------------------------------------------------------

    #[test]
    fn unlink_link_not_found() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &[]);
        make_store_with_ticket(tmp.path(), "task-0002", &[]);

        let err = unlink_impl(Some(tmp.path()), "task-0001", "task-0002").unwrap_err();
        assert!(
            matches!(err, Error::LinkNotFound),
            "expected LinkNotFound, got {err:?}"
        );
        assert_eq!(err.to_string(), "Link not found");
    }

    // -----------------------------------------------------------------------
    // unlink — one-sided link (inconsistent state) should also fail
    // -----------------------------------------------------------------------

    #[test]
    fn unlink_one_sided_link_not_found() {
        let tmp = tempdir().unwrap();
        // task-0001 points to task-0002, but task-0002 does not point back.
        make_store_with_ticket(tmp.path(), "task-0001", &["task-0002"]);
        make_store_with_ticket(tmp.path(), "task-0002", &[]);

        let err = unlink_impl(Some(tmp.path()), "task-0001", "task-0002").unwrap_err();
        assert!(
            matches!(err, Error::LinkNotFound),
            "expected LinkNotFound for one-sided link, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Non-existent ticket
    // -----------------------------------------------------------------------

    #[test]
    fn link_nonexistent_ticket() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &[]);

        let err = link_impl(Some(tmp.path()), &["task-0001", "nonexistent"]).unwrap_err();
        assert!(
            matches!(err, Error::TicketNotFound { .. }),
            "expected TicketNotFound, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Partial ID resolution
    // -----------------------------------------------------------------------

    #[test]
    fn link_partial_id_resolution() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &[]);
        make_store_with_ticket(tmp.path(), "task-0002", &[]);

        // Use suffix-only partial IDs.
        link_impl(Some(tmp.path()), &["0001", "0002"]).unwrap();

        assert!(read_links(tmp.path(), "task-0001").contains(&"task-0002".to_string()));
        assert!(read_links(tmp.path(), "task-0002").contains(&"task-0001".to_string()));
    }
}
