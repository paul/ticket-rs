// dep, undep, and dep tree commands.
//
// `dep add` / `dep remove` add and remove entries from a ticket's deps array.
// Both resolve both the source and dependency IDs via partial matching, then
// read the source ticket, mutate its deps field, and write it back using the
// existing round-trip serializer.  All other frontmatter fields and the body
// are preserved exactly.
//
// `dep tree` walks the dependency graph recursively from the root ticket and
// renders it as an ASCII tree using box-drawing characters.  Each node shows
// the ticket ID, status, and title.  By default, nodes are deduplicated: each
// ticket is shown only at its deepest nesting.  The `--full` flag disables
// deduplication.  Circular dependencies are detected and annotated with
// `[cycle]` rather than causing infinite loops.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::error::{Error, Result};
use crate::pager;
use crate::store::TicketStore;
use crate::ticket::Ticket;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Extract the ticket ID from a resolved `.md` path.
///
/// The path always originates from `TicketStore::resolve_id`, which only
/// returns paths whose stems are valid UTF-8 ticket IDs.  The `partial`
/// fallback is therefore unreachable in practice; it exists only to satisfy
/// the type system without an unwrap.
fn full_id_from_path<'a>(path: &'a Path, partial: &'a str) -> &'a str {
    debug_assert!(
        path.file_stem().and_then(|s| s.to_str()).is_some(),
        "ticket path stem should always be valid UTF-8: {path:?}"
    );
    path.file_stem().and_then(|s| s.to_str()).unwrap_or(partial)
}

/// Add `dep_id` to the deps of ticket `id`.  Returns the output message.
fn dep_impl(start_dir: Option<&Path>, partial_id: &str, partial_dep_id: &str) -> Result<String> {
    let store = TicketStore::find(start_dir)?;

    // Resolve both IDs — either missing ticket is a TicketNotFound error.
    let dep_path = store.resolve_id(partial_dep_id)?;
    let dep_full_id = full_id_from_path(&dep_path, partial_dep_id);

    let src_path = store.resolve_id(partial_id)?;
    let src_full_id = full_id_from_path(&src_path, partial_id);

    let mut ticket = store.read_ticket(src_full_id)?;

    if ticket.deps.iter().any(|d| d == dep_full_id) {
        return Ok("Dependency already exists".to_string());
    }

    ticket.deps.push(dep_full_id.to_string());
    store.write_ticket(&ticket)?;

    Ok(format!("Added dependency: {src_full_id} -> {dep_full_id}"))
}

/// Remove `dep_id` from the deps of ticket `id`.  Returns the output message,
/// or `Error::DependencyNotFound` if the dependency is not present.
fn undep_impl(start_dir: Option<&Path>, partial_id: &str, partial_dep_id: &str) -> Result<String> {
    let store = TicketStore::find(start_dir)?;

    let dep_path = store.resolve_id(partial_dep_id)?;
    let dep_full_id = full_id_from_path(&dep_path, partial_dep_id);

    let src_path = store.resolve_id(partial_id)?;
    let src_full_id = full_id_from_path(&src_path, partial_id);

    let mut ticket = store.read_ticket(src_full_id)?;

    let pos = ticket
        .deps
        .iter()
        .position(|d| d == dep_full_id)
        .ok_or(Error::DependencyNotFound)?;

    ticket.deps.remove(pos);
    store.write_ticket(&ticket)?;

    Ok(format!(
        "Removed dependency: {src_full_id} -/-> {dep_full_id}"
    ))
}

// ---------------------------------------------------------------------------
// dep tree implementation
// ---------------------------------------------------------------------------

/// Compute the maximum depth of the subtree rooted at `id`, where depth is the
/// number of edges to the deepest leaf.  A node with no deps has depth 0.
/// The `ancestors` set is used to break cycles; a dependency that is an
/// ancestor is treated as a leaf (depth 0).
fn subtree_depth(
    id: &str,
    tickets: &HashMap<String, Ticket>,
    ancestors: &mut HashSet<String>,
    memo: &mut HashMap<String, usize>,
) -> usize {
    if let Some(&cached) = memo.get(id) {
        return cached;
    }
    let Some(ticket) = tickets.get(id) else {
        return 0;
    };
    if ticket.deps.is_empty() {
        memo.insert(id.to_string(), 0);
        return 0;
    }
    ancestors.insert(id.to_string());
    let deps: Vec<String> = ticket
        .deps
        .iter()
        .filter(|dep| !ancestors.contains(*dep))
        .cloned()
        .collect();
    let max_child = deps
        .iter()
        .map(|dep| subtree_depth(dep, tickets, ancestors, memo))
        .max()
        .unwrap_or(0);
    ancestors.remove(id);
    let depth = 1 + max_child;
    memo.insert(id.to_string(), depth);
    depth
}

/// Compute the set of IDs that will appear at deeper nesting elsewhere in the
/// tree.  This is used in default (dedup) mode to skip a node at a shallower
/// level when it will be rendered at a deeper one.
///
/// The algorithm: for every node reachable in the tree, if it is reachable via
/// more than one path (i.e., it appears as a dep of multiple nodes), track the
/// maximum depth at which it appears.  Nodes should only be rendered at that
/// maximum depth; at all shallower depths they are suppressed.
///
/// We perform a DFS and record `(id, current_depth)` for every node visit.
/// After the full traversal, any node that has been seen at more than one
/// depth is a candidate for dedup; we keep only the deepest visit.
fn compute_max_depths(
    id: &str,
    current_depth: usize,
    tickets: &HashMap<String, Ticket>,
    ancestors: &HashSet<String>,
    depths: &mut HashMap<String, usize>,
) {
    let entry = depths.entry(id.to_string()).or_insert(0);
    if current_depth > *entry {
        *entry = current_depth;
    }

    let Some(ticket) = tickets.get(id) else {
        return;
    };

    let mut new_ancestors = ancestors.clone();
    new_ancestors.insert(id.to_string());

    for dep_id in &ticket.deps {
        if ancestors.contains(dep_id.as_str()) {
            // Cycle — don't recurse.
            continue;
        }
        compute_max_depths(dep_id, current_depth + 1, tickets, &new_ancestors, depths);
    }
}

/// A single rendered line of the tree.
struct TreeLine {
    /// The rendered text (prefix + connector + content).
    text: String,
}

/// Recursively render the dependency tree rooted at `id` into `lines`.
///
/// - `id` — current node's ticket ID
/// - `prefix` — prefix string prepended before the connector (built up from parent connectors)
/// - `is_last` — whether this node is the last child of its parent
/// - `is_root` — whether this node is the root (no connector)
/// - `tickets` — all known tickets
/// - `ancestors` — IDs of the current DFS path (for cycle detection)
/// - `visited` — IDs already rendered (for dedup in non-full mode)
/// - `full` — if true, dedup is disabled
/// - `max_depths` — for dedup: the maximum depth at which each ID appears
/// - `depth` — current depth from root
/// - `depth_memo` — memoized subtree depths for sorting
#[allow(clippy::too_many_arguments)]
fn render_node(
    id: &str,
    prefix: &str,
    is_last: bool,
    is_root: bool,
    tickets: &HashMap<String, Ticket>,
    ancestors: &mut HashSet<String>,
    visited: &mut HashSet<String>,
    full: bool,
    max_depths: &HashMap<String, usize>,
    depth: usize,
    depth_memo: &mut HashMap<String, usize>,
    lines: &mut Vec<TreeLine>,
) {
    // Determine the node's display content.
    let (status_str, title_str) = match tickets.get(id) {
        Some(t) => (format!("{}", t.status), t.title.clone()),
        None => ("unknown".to_string(), "(not found)".to_string()),
    };

    // Build the connector and new child prefix.
    let (connector, child_prefix) = if is_root {
        ("".to_string(), "".to_string())
    } else if is_last {
        ("└── ".to_string(), format!("{prefix}    "))
    } else {
        ("├── ".to_string(), format!("{prefix}│   "))
    };

    // If this node is already on the current ancestor path, annotate it as a
    // cycle and stop recursing.  We check before building the normal line so
    // the correct text is rendered directly rather than push-then-replace.
    if ancestors.contains(id) {
        lines.push(TreeLine {
            text: format!("{prefix}{connector}{id} [cycle]"),
        });
        return;
    }

    let line_text = format!("{prefix}{connector}{id} [{status_str}] {title_str}");
    lines.push(TreeLine { text: line_text });

    // In dedup mode, skip if this node's maximum depth is greater than the
    // current depth (meaning it will appear deeper elsewhere).
    if !full && !is_root && max_depths.get(id).is_some_and(|&max_d| max_d > depth) {
        // This node will appear at a deeper level.  Remove the line we just added.
        lines.pop();
        return;
    }

    // Mark as visited.
    visited.insert(id.to_string());

    let Some(ticket) = tickets.get(id) else {
        return;
    };

    if ticket.deps.is_empty() {
        return;
    }

    // Sort children: primary key = subtree depth (ascending, shallow first),
    // secondary key = ticket ID (lexicographic).
    let mut children: Vec<&String> = ticket.deps.iter().collect();
    ancestors.insert(id.to_string());
    children.sort_by(|a, b| {
        let da = subtree_depth(a, tickets, &mut ancestors.clone(), depth_memo);
        let db = subtree_depth(b, tickets, &mut ancestors.clone(), depth_memo);
        da.cmp(&db).then_with(|| a.cmp(b))
    });

    let child_count = children.len();
    for (i, child_id) in children.iter().enumerate() {
        let child_is_last = i == child_count - 1;
        let is_cycle = ancestors.contains(child_id.as_str());

        if !full && !is_cycle {
            // In dedup mode, also skip if already visited AND the node will
            // not appear at this depth (already emitted from another branch).
            if visited.contains(child_id.as_str()) {
                continue;
            }
        }

        render_node(
            child_id,
            &child_prefix,
            child_is_last,
            false,
            tickets,
            ancestors,
            visited,
            full,
            max_depths,
            depth + 1,
            depth_memo,
            lines,
        );
    }
    ancestors.remove(id);
}

/// Build and return the rendered dependency tree string for `partial_id`.
fn dep_tree_impl(start_dir: Option<&Path>, partial_id: &str, full: bool) -> Result<String> {
    let store = TicketStore::find(start_dir)?;

    // Resolve the root ticket ID.
    let root_path = store.resolve_id(partial_id)?;
    let root_id = full_id_from_path(&root_path, partial_id).to_string();

    // Load all tickets into a map for fast lookup.
    let tickets: HashMap<String, Ticket> = store
        .list_tickets()
        .into_iter()
        .map(|t| (t.id.clone(), t))
        .collect();

    // Verify the root ticket exists (list_tickets skips parse errors).
    if !tickets.contains_key(&root_id) {
        return Err(Error::TicketNotFound { id: root_id });
    }

    // Pre-compute the maximum depth at which each node appears in the tree,
    // used for deduplication.
    let mut max_depths: HashMap<String, usize> = HashMap::new();
    compute_max_depths(&root_id, 0, &tickets, &HashSet::new(), &mut max_depths);

    // Pre-compute subtree depths for sorting.
    let mut depth_memo: HashMap<String, usize> = HashMap::new();

    let mut ancestors: HashSet<String> = HashSet::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut lines: Vec<TreeLine> = Vec::new();

    render_node(
        &root_id,
        "",
        true,
        true,
        &tickets,
        &mut ancestors,
        &mut visited,
        full,
        &max_depths,
        0,
        &mut depth_memo,
        &mut lines,
    );

    let output = lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    Ok(output)
}

// ---------------------------------------------------------------------------
// dep cycle implementation
// ---------------------------------------------------------------------------

/// DFS node color for cycle detection (three-color algorithm).
#[derive(Clone, PartialEq)]
enum Color {
    /// Not yet visited.
    White,
    /// Currently on the DFS stack (in-progress).
    Gray,
    /// Fully processed.
    Black,
}

/// A single detected cycle, ready for display.
struct Cycle {
    /// The chain string, e.g. "a -> b -> c -> a".
    chain: String,
    /// The IDs of members, in normalized rotation order.
    members: Vec<String>,
}

/// Run DFS from `node`, using `color` for three-coloring and `path` to track
/// the current stack.  All back-edges encountered anywhere in the subtree are
/// collected into `found` as cycle chain strings.  Traversal continues after
/// each back-edge so that sibling branches are also explored.
fn dfs_find_cycles(
    node: &str,
    tickets: &HashMap<String, Ticket>,
    color: &mut HashMap<String, Color>,
    path: &mut Vec<String>,
    found: &mut Vec<String>,
) {
    // Node not in graph (closed/unknown dep) — treat as fully visited.
    if !tickets.contains_key(node) {
        return;
    }

    match color.get(node).cloned().unwrap_or(Color::White) {
        // Already fully processed — no new cycles reachable here.
        Color::Black => return,
        // Back edge — record the cycle and return without recursing further
        // (the cycle members are already on the stack; recursing would only
        // find the same cycle again or loop forever).
        Color::Gray => {
            let start_pos = path.iter().position(|id| id == node).unwrap_or(0);
            let cycle_nodes: Vec<&str> = path[start_pos..].iter().map(String::as_str).collect();
            let mut chain = cycle_nodes.join(" -> ");
            chain.push_str(&format!(" -> {node}"));
            found.push(chain);
            return;
        }
        Color::White => {}
    }

    // Mark gray and push onto path.
    color.insert(node.to_string(), Color::Gray);
    path.push(node.to_string());

    let deps: Vec<String> = tickets
        .get(node)
        .map(|t| t.deps.clone())
        .unwrap_or_default();

    // Visit every dependency — do not break early so all back-edges are found.
    for dep in &deps {
        dfs_find_cycles(dep, tickets, color, path, found);
    }

    // Pop from path and mark black (fully processed).
    path.pop();
    color.insert(node.to_string(), Color::Black);
}

/// Normalize a cycle chain so that the node with the lexicographically smallest
/// ID comes first.  Returns the normalized member IDs (without the repeated
/// tail) used for deduplication.
fn normalize_cycle(chain: &str) -> Vec<String> {
    // The chain is "a -> b -> c -> a"; split on " -> " to get members including
    // the repeated tail.
    let parts: Vec<&str> = chain.split(" -> ").collect();
    // The last element is a duplicate of the first, so the unique members are
    // everything except the last part.
    let n = parts.len() - 1;
    if n == 0 {
        return vec![];
    }
    let members: Vec<&str> = parts[..n].to_vec();

    // Find the position of the lexicographically smallest ID.
    let min_pos = members
        .iter()
        .enumerate()
        .min_by_key(|(_, id)| *id)
        .map(|(i, _)| i)
        .unwrap_or(0);

    // Rotate so smallest ID is first.
    members[min_pos..]
        .iter()
        .chain(members[..min_pos].iter())
        .map(|s| s.to_string())
        .collect()
}

/// Build and return the cycle detection output string and whether any cycles
/// were found.
fn dep_cycle_impl(start_dir: Option<&Path>) -> Result<(String, bool)> {
    let store = TicketStore::find(start_dir)?;

    // Load only open and in_progress tickets.
    let tickets: HashMap<String, Ticket> = store
        .list_tickets()
        .into_iter()
        .filter(|t| {
            matches!(
                t.status,
                crate::ticket::Status::Open | crate::ticket::Status::InProgress
            )
        })
        .map(|t| (t.id.clone(), t))
        .collect();

    let mut color: HashMap<String, Color> = HashMap::new();
    let mut seen_cycles: HashSet<String> = HashSet::new();
    let mut cycles: Vec<Cycle> = Vec::new();

    // Sort node IDs for deterministic traversal order.
    let mut node_ids: Vec<String> = tickets.keys().cloned().collect();
    node_ids.sort();

    for id in &node_ids {
        if color.get(id).cloned().unwrap_or(Color::White) == Color::White {
            let mut path: Vec<String> = Vec::new();
            let mut raw_cycles: Vec<String> = Vec::new();
            dfs_find_cycles(id, &tickets, &mut color, &mut path, &mut raw_cycles);
            for chain in raw_cycles {
                let members = normalize_cycle(&chain);
                // Use sorted, comma-joined members as dedup key.
                let key = {
                    let mut sorted = members.clone();
                    sorted.sort();
                    sorted.join(",")
                };
                if !seen_cycles.contains(&key) {
                    seen_cycles.insert(key);
                    cycles.push(Cycle { chain, members });
                }
            }
        }
    }

    if cycles.is_empty() {
        return Ok(("No dependency cycles found".to_string(), false));
    }

    // Build the output string.
    let mut parts: Vec<String> = Vec::new();
    for (i, cycle) in cycles.iter().enumerate() {
        let mut block = format!("Cycle {}: {}", i + 1, cycle.chain);
        for member_id in &cycle.members {
            if let Some(t) = tickets.get(member_id) {
                block.push_str(&format!("\n  {:<8} [{}] {}", member_id, t.status, t.title));
            }
        }
        parts.push(block);
    }

    Ok((parts.join("\n\n"), true))
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

pub fn dep(id: &str, dep_id: &str) -> Result<()> {
    let msg = dep_impl(None, id, dep_id)?;
    println!("{msg}");
    Ok(())
}

pub fn dep_remove(id: &str, dep_id: &str) -> Result<()> {
    let msg = undep_impl(None, id, dep_id)?;
    println!("{msg}");
    Ok(())
}

pub fn dep_tree(id: &str, full: bool) -> Result<()> {
    let output = dep_tree_impl(None, id, full)?;
    pager::page_or_print(&format!("{output}\n"))
}

/// Run cycle detection and print the result.  Exits with code 1 if cycles are
/// found, or 0 if the graph is clean.
pub fn dep_cycle() -> Result<()> {
    let (output, has_cycles) = dep_cycle_impl(None)?;
    println!("{output}");
    if has_cycles {
        std::process::exit(1);
    }
    Ok(())
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

    /// Write a minimal ticket file to a temp .tickets/ dir and return the store.
    fn make_store_with_ticket(dir: &Path, id: &str, deps: &[&str]) -> TicketStore {
        make_store_with_ticket_titled(dir, id, "A ticket", deps)
    }

    fn make_store_with_ticket_titled(
        dir: &Path,
        id: &str,
        title: &str,
        deps: &[&str],
    ) -> TicketStore {
        make_store_with_ticket_status(dir, id, title, "open", deps)
    }

    fn make_store_with_ticket_status(
        dir: &Path,
        id: &str,
        title: &str,
        status: &str,
        deps: &[&str],
    ) -> TicketStore {
        let tickets_dir = dir.join(".tickets");
        fs::create_dir_all(&tickets_dir).unwrap();

        let deps_str = deps.join(", ");
        let content = format!(
            "---\nid: {id}\nstatus: {status}\ndeps: [{deps_str}]\nlinks: []\ncreated: 2026-01-01T00:00:00Z\ntype: task\npriority: 2\nassignee: Test User\ntags: [phase-2]\n---\n# {title}\n\nBody text.\n"
        );
        fs::write(tickets_dir.join(format!("{id}.md")), &content).unwrap();

        TicketStore::find(Some(dir)).unwrap()
    }

    fn read_deps(dir: &Path, id: &str) -> Vec<String> {
        let path = dir.join(".tickets").join(format!("{id}.md"));
        let content = fs::read_to_string(path).unwrap();
        let line = content.lines().find(|l| l.starts_with("deps: ")).unwrap();
        // Parse "deps: [a, b, c]" → vec!["a", "b", "c"]
        let inner = line.trim_start_matches("deps: [").trim_end_matches(']');
        if inner.is_empty() {
            vec![]
        } else {
            inner.split(", ").map(|s| s.to_string()).collect()
        }
    }

    // -----------------------------------------------------------------------
    // dep adds dependency
    // -----------------------------------------------------------------------

    #[test]
    fn dep_adds_dependency() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &[]);
        make_store_with_ticket(tmp.path(), "task-0002", &[]);
        dep_impl(Some(tmp.path()), "task-0001", "task-0002").unwrap();
        assert!(read_deps(tmp.path(), "task-0001").contains(&"task-0002".to_string()));
    }

    // -----------------------------------------------------------------------
    // dep is idempotent
    // -----------------------------------------------------------------------

    #[test]
    fn dep_is_idempotent() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &[]);
        make_store_with_ticket(tmp.path(), "task-0002", &[]);
        dep_impl(Some(tmp.path()), "task-0001", "task-0002").unwrap();
        dep_impl(Some(tmp.path()), "task-0001", "task-0002").unwrap();
        let deps = read_deps(tmp.path(), "task-0001");
        assert_eq!(
            deps.iter().filter(|d| *d == "task-0002").count(),
            1,
            "task-0002 should appear exactly once in deps"
        );
    }

    // -----------------------------------------------------------------------
    // dep output message
    // -----------------------------------------------------------------------

    #[test]
    fn dep_output_message() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &[]);
        make_store_with_ticket(tmp.path(), "task-0002", &[]);
        let msg = dep_impl(Some(tmp.path()), "task-0001", "task-0002").unwrap();
        assert_eq!(msg, "Added dependency: task-0001 -> task-0002");
    }

    // -----------------------------------------------------------------------
    // dep — already exists output
    // -----------------------------------------------------------------------

    #[test]
    fn dep_already_exists_output() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &["task-0002"]);
        make_store_with_ticket(tmp.path(), "task-0002", &[]);
        let msg = dep_impl(Some(tmp.path()), "task-0001", "task-0002").unwrap();
        assert_eq!(msg, "Dependency already exists");
    }

    // -----------------------------------------------------------------------
    // undep removes dependency
    // -----------------------------------------------------------------------

    #[test]
    fn undep_removes_dependency() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &["task-0002"]);
        make_store_with_ticket(tmp.path(), "task-0002", &[]);
        undep_impl(Some(tmp.path()), "task-0001", "task-0002").unwrap();
        assert!(!read_deps(tmp.path(), "task-0001").contains(&"task-0002".to_string()));
    }

    // -----------------------------------------------------------------------
    // undep output message
    // -----------------------------------------------------------------------

    #[test]
    fn undep_output_message() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &["task-0002"]);
        make_store_with_ticket(tmp.path(), "task-0002", &[]);
        let msg = undep_impl(Some(tmp.path()), "task-0001", "task-0002").unwrap();
        assert_eq!(msg, "Removed dependency: task-0001 -/-> task-0002");
    }

    // -----------------------------------------------------------------------
    // undep — not found error
    // -----------------------------------------------------------------------

    #[test]
    fn undep_not_found_error() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &[]);
        make_store_with_ticket(tmp.path(), "task-0002", &[]);
        let err = undep_impl(Some(tmp.path()), "task-0001", "task-0002").unwrap_err();
        assert!(
            matches!(err, Error::DependencyNotFound),
            "expected DependencyNotFound, got {err:?}"
        );
        assert_eq!(err.to_string(), "Dependency not found");
    }

    // -----------------------------------------------------------------------
    // dep — non-existent dep ticket
    // -----------------------------------------------------------------------

    #[test]
    fn dep_nonexistent_dep_ticket() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &[]);
        let err = dep_impl(Some(tmp.path()), "task-0001", "nonexistent").unwrap_err();
        assert!(
            matches!(err, Error::TicketNotFound { .. }),
            "expected TicketNotFound, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // dep — non-existent source ticket
    // -----------------------------------------------------------------------

    #[test]
    fn dep_nonexistent_source_ticket() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0002", &[]);
        let err = dep_impl(Some(tmp.path()), "nonexistent", "task-0002").unwrap_err();
        assert!(
            matches!(err, Error::TicketNotFound { .. }),
            "expected TicketNotFound, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Partial ID resolution
    // -----------------------------------------------------------------------

    #[test]
    fn partial_id_resolution() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &[]);
        make_store_with_ticket(tmp.path(), "task-0002", &[]);
        // Use suffix-only partial IDs for both arguments.
        dep_impl(Some(tmp.path()), "0001", "0002").unwrap();
        assert!(read_deps(tmp.path(), "task-0001").contains(&"task-0002".to_string()));
    }

    // -----------------------------------------------------------------------
    // Frontmatter preserved after dep
    // -----------------------------------------------------------------------

    #[test]
    fn frontmatter_preserved() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &[]);
        make_store_with_ticket(tmp.path(), "task-0002", &[]);

        let before = fs::read_to_string(tmp.path().join(".tickets").join("task-0001.md")).unwrap();

        dep_impl(Some(tmp.path()), "task-0001", "task-0002").unwrap();

        let after = fs::read_to_string(tmp.path().join(".tickets").join("task-0001.md")).unwrap();

        // Only the deps line should differ.
        assert_eq!(
            before.replace("deps: []", "deps: [task-0002]"),
            after,
            "file content differed beyond the deps line"
        );
        assert!(after.contains("assignee: Test User"), "assignee was lost");
        assert!(after.contains("tags: [phase-2]"), "tags were lost");
        assert!(after.contains("Body text."), "body was lost");
    }

    // -----------------------------------------------------------------------
    // dep tree — linear chain A → B → C
    // -----------------------------------------------------------------------

    #[test]
    fn tree_linear_chain() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &["task-0002"]);
        make_store_with_ticket(tmp.path(), "task-0002", &["task-0003"]);
        make_store_with_ticket(tmp.path(), "task-0003", &[]);

        let output = dep_tree_impl(Some(tmp.path()), "task-0001", false).unwrap();

        assert!(output.contains("task-0001"), "missing task-0001");
        assert!(output.contains("task-0002"), "missing task-0002");
        assert!(output.contains("task-0003"), "missing task-0003");

        // Box-drawing characters must be present.
        assert!(
            output.contains("├──") || output.contains("└──"),
            "missing box-drawing characters: {output}"
        );
    }

    // -----------------------------------------------------------------------
    // dep tree — each node shows status and title
    // -----------------------------------------------------------------------

    #[test]
    fn tree_shows_status_and_title() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket_titled(tmp.path(), "task-0001", "Main task", &["task-0002"]);
        make_store_with_ticket_titled(tmp.path(), "task-0002", "Dependency task", &[]);

        let output = dep_tree_impl(Some(tmp.path()), "task-0001", false).unwrap();

        assert!(output.contains("[open]"), "missing [open] status");
        assert!(output.contains("Main task"), "missing root title");
        assert!(output.contains("Dependency task"), "missing dep title");
    }

    // -----------------------------------------------------------------------
    // dep tree — multiple direct deps
    // -----------------------------------------------------------------------

    #[test]
    fn tree_multiple_direct_deps() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &["task-0002", "task-0003"]);
        make_store_with_ticket(tmp.path(), "task-0002", &[]);
        make_store_with_ticket(tmp.path(), "task-0003", &[]);

        let output = dep_tree_impl(Some(tmp.path()), "task-0001", false).unwrap();

        assert!(output.contains("task-0002"), "missing task-0002");
        assert!(output.contains("task-0003"), "missing task-0003");
    }

    // -----------------------------------------------------------------------
    // dep tree — deduplication (default): A → B, A → C, B → C; C once
    // -----------------------------------------------------------------------

    #[test]
    fn tree_deduplication_default() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &["task-0002", "task-0003"]);
        make_store_with_ticket(tmp.path(), "task-0002", &["task-0003"]);
        make_store_with_ticket(tmp.path(), "task-0003", &[]);

        let output = dep_tree_impl(Some(tmp.path()), "task-0001", false).unwrap();

        let count = output.matches("task-0003").count();
        assert_eq!(
            count, 1,
            "task-0003 should appear exactly once; output:\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // dep tree — --full disables dedup: A → B, A → C, B → C; C twice
    // -----------------------------------------------------------------------

    #[test]
    fn tree_full_disables_dedup() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &["task-0002", "task-0003"]);
        make_store_with_ticket(tmp.path(), "task-0002", &["task-0003"]);
        make_store_with_ticket(tmp.path(), "task-0003", &[]);

        let output = dep_tree_impl(Some(tmp.path()), "task-0001", true).unwrap();

        let count = output.matches("task-0003").count();
        assert_eq!(
            count, 2,
            "task-0003 should appear twice with --full; output:\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // dep tree — cycle detection: A → B → A; no infinite loop, [cycle] shown
    // -----------------------------------------------------------------------

    #[test]
    fn tree_cycle_detection_no_infinite_loop() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &["task-0002"]);
        make_store_with_ticket(tmp.path(), "task-0002", &["task-0001"]);

        // Must terminate and not panic.
        let output = dep_tree_impl(Some(tmp.path()), "task-0001", false).unwrap();

        assert!(output.contains("task-0001"), "missing task-0001");
        assert!(output.contains("task-0002"), "missing task-0002");
        assert!(
            output.contains("[cycle]"),
            "missing [cycle] annotation; output:\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // dep tree — sorting by subtree depth then ID
    // -----------------------------------------------------------------------

    #[test]
    fn tree_sorting_depth_then_id() {
        // task-0001 → {task-0002 (shallow), task-0003 (shallow), task-0004 (deep)}
        // task-0004 → task-0005
        // Expected order: task-0002, task-0003 (shallow, id order), task-0004 (deep)
        let tmp = tempdir().unwrap();
        make_store_with_ticket(
            tmp.path(),
            "task-0001",
            &["task-0002", "task-0003", "task-0004"],
        );
        make_store_with_ticket(tmp.path(), "task-0002", &[]);
        make_store_with_ticket(tmp.path(), "task-0003", &[]);
        make_store_with_ticket(tmp.path(), "task-0004", &["task-0005"]);
        make_store_with_ticket(tmp.path(), "task-0005", &[]);

        let output = dep_tree_impl(Some(tmp.path()), "task-0001", false).unwrap();

        let pos2 = output.find("task-0002").unwrap();
        let pos3 = output.find("task-0003").unwrap();
        let pos4 = output.find("task-0004").unwrap();

        assert!(
            pos2 < pos3,
            "task-0002 should come before task-0003; output:\n{output}"
        );
        assert!(
            pos3 < pos4,
            "task-0003 should come before task-0004; output:\n{output}"
        );
        assert!(
            pos2 < pos4,
            "task-0002 should come before task-0004; output:\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // dep tree — sorting by ID when same depth
    // -----------------------------------------------------------------------

    #[test]
    fn tree_sorting_same_depth_by_id() {
        // All children of task-0001 are leaves (depth 0).  They should sort by ID.
        let tmp = tempdir().unwrap();
        make_store_with_ticket(
            tmp.path(),
            "task-0001",
            &["task-0005", "task-0002", "task-0004", "task-0003"],
        );
        make_store_with_ticket(tmp.path(), "task-0002", &[]);
        make_store_with_ticket(tmp.path(), "task-0003", &[]);
        make_store_with_ticket(tmp.path(), "task-0004", &[]);
        make_store_with_ticket(tmp.path(), "task-0005", &[]);

        let output = dep_tree_impl(Some(tmp.path()), "task-0001", false).unwrap();

        let pos2 = output.find("task-0002").unwrap();
        let pos3 = output.find("task-0003").unwrap();
        let pos4 = output.find("task-0004").unwrap();
        let pos5 = output.find("task-0005").unwrap();

        assert!(pos2 < pos3, "task-0002 before task-0003; output:\n{output}");
        assert!(pos3 < pos4, "task-0003 before task-0004; output:\n{output}");
        assert!(pos4 < pos5, "task-0004 before task-0005; output:\n{output}");
    }

    // -----------------------------------------------------------------------
    // dep tree — partial ID for root
    // -----------------------------------------------------------------------

    #[test]
    fn tree_partial_id_for_root() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket_titled(tmp.path(), "task-0001", "Main task", &[]);

        // Use suffix as partial ID.
        let output = dep_tree_impl(Some(tmp.path()), "0001", false).unwrap();
        assert!(output.contains("task-0001"), "output:\n{output}");
    }

    // -----------------------------------------------------------------------
    // dep tree — non-existent root ticket → TicketNotFound
    // -----------------------------------------------------------------------

    #[test]
    fn tree_nonexistent_root() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &[]);

        let err = dep_tree_impl(Some(tmp.path()), "nonexistent", false).unwrap_err();
        assert!(
            matches!(err, Error::TicketNotFound { .. }),
            "expected TicketNotFound, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // dep cycle — no cycles exits clean
    // -----------------------------------------------------------------------

    #[test]
    fn cycle_no_cycles() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &["task-0002"]);
        make_store_with_ticket(tmp.path(), "task-0002", &["task-0003"]);
        make_store_with_ticket(tmp.path(), "task-0003", &[]);

        let (output, has_cycles) = dep_cycle_impl(Some(tmp.path())).unwrap();
        assert!(!has_cycles, "expected no cycles; output:\n{output}");
        assert_eq!(output, "No dependency cycles found");
    }

    // -----------------------------------------------------------------------
    // dep cycle — simple two-node cycle A → B → A
    // -----------------------------------------------------------------------

    #[test]
    fn cycle_two_node() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &["task-0002"]);
        make_store_with_ticket(tmp.path(), "task-0002", &["task-0001"]);

        let (output, has_cycles) = dep_cycle_impl(Some(tmp.path())).unwrap();
        assert!(has_cycles, "expected a cycle; output:\n{output}");
        assert!(
            output.contains("task-0001"),
            "missing task-0001; output:\n{output}"
        );
        assert!(
            output.contains("task-0002"),
            "missing task-0002; output:\n{output}"
        );
        // Only one cycle should be reported.
        assert_eq!(
            output.matches("Cycle ").count(),
            1,
            "expected exactly one cycle block; output:\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // dep cycle — three-node cycle A → B → C → A
    // -----------------------------------------------------------------------

    #[test]
    fn cycle_three_node() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &["task-0002"]);
        make_store_with_ticket(tmp.path(), "task-0002", &["task-0003"]);
        make_store_with_ticket(tmp.path(), "task-0003", &["task-0001"]);

        let (output, has_cycles) = dep_cycle_impl(Some(tmp.path())).unwrap();
        assert!(has_cycles, "expected a cycle; output:\n{output}");
        assert!(
            output.contains("task-0001"),
            "missing task-0001; output:\n{output}"
        );
        assert!(
            output.contains("task-0002"),
            "missing task-0002; output:\n{output}"
        );
        assert!(
            output.contains("task-0003"),
            "missing task-0003; output:\n{output}"
        );
        assert_eq!(
            output.matches("Cycle ").count(),
            1,
            "expected exactly one cycle block; output:\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // dep cycle — multiple independent cycles A↔B and C↔D
    // -----------------------------------------------------------------------

    #[test]
    fn cycle_multiple_independent() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &["task-0002"]);
        make_store_with_ticket(tmp.path(), "task-0002", &["task-0001"]);
        make_store_with_ticket(tmp.path(), "task-0003", &["task-0004"]);
        make_store_with_ticket(tmp.path(), "task-0004", &["task-0003"]);

        let (output, has_cycles) = dep_cycle_impl(Some(tmp.path())).unwrap();
        assert!(has_cycles, "expected cycles; output:\n{output}");
        assert_eq!(
            output.matches("Cycle ").count(),
            2,
            "expected exactly two cycle blocks; output:\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // dep cycle — cycle normalization: same cycle deduplicated regardless of
    // which node the DFS starts from
    // -----------------------------------------------------------------------

    #[test]
    fn cycle_normalization_deduplicates() {
        let tmp = tempdir().unwrap();
        // A → B → C → A — single cycle regardless of traversal start.
        make_store_with_ticket(tmp.path(), "task-0001", &["task-0002"]);
        make_store_with_ticket(tmp.path(), "task-0002", &["task-0003"]);
        make_store_with_ticket(tmp.path(), "task-0003", &["task-0001"]);

        let (output, has_cycles) = dep_cycle_impl(Some(tmp.path())).unwrap();
        assert!(has_cycles, "expected a cycle; output:\n{output}");
        // Regardless of which node was the DFS entry point, only one cycle
        // block should appear.
        assert_eq!(
            output.matches("Cycle ").count(),
            1,
            "expected exactly one cycle block (dedup); output:\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // dep cycle — closed tickets are skipped
    // -----------------------------------------------------------------------

    #[test]
    fn cycle_closed_tickets_skipped() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket_status(tmp.path(), "task-0001", "A ticket", "open", &["task-0002"]);
        // task-0002 is closed, so the cycle A→B→A should not be detected.
        make_store_with_ticket_status(
            tmp.path(),
            "task-0002",
            "A ticket",
            "closed",
            &["task-0001"],
        );

        let (output, has_cycles) = dep_cycle_impl(Some(tmp.path())).unwrap();
        assert!(
            !has_cycles,
            "expected no cycles (closed ticket skipped); output:\n{output}"
        );
        assert_eq!(output, "No dependency cycles found");
    }

    // -----------------------------------------------------------------------
    // dep cycle — in_progress tickets are included
    // -----------------------------------------------------------------------

    #[test]
    fn cycle_in_progress_included() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket_status(
            tmp.path(),
            "task-0001",
            "A ticket",
            "in_progress",
            &["task-0002"],
        );
        make_store_with_ticket_status(tmp.path(), "task-0002", "A ticket", "open", &["task-0001"]);

        let (output, has_cycles) = dep_cycle_impl(Some(tmp.path())).unwrap();
        assert!(
            has_cycles,
            "expected cycle with in_progress ticket; output:\n{output}"
        );
        assert!(
            output.contains("task-0001"),
            "missing task-0001; output:\n{output}"
        );
        assert!(
            output.contains("task-0002"),
            "missing task-0002; output:\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // dep cycle — non-cyclic dep chain A → B → C
    // -----------------------------------------------------------------------

    #[test]
    fn cycle_linear_chain_no_cycle() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket(tmp.path(), "task-0001", &["task-0002"]);
        make_store_with_ticket(tmp.path(), "task-0002", &["task-0003"]);
        make_store_with_ticket(tmp.path(), "task-0003", &[]);

        let (output, has_cycles) = dep_cycle_impl(Some(tmp.path())).unwrap();
        assert!(!has_cycles, "expected no cycles; output:\n{output}");
    }

    // -----------------------------------------------------------------------
    // dep cycle — overlapping cycles sharing a node (A->B->A and A->C->A)
    // -----------------------------------------------------------------------

    #[test]
    fn cycle_overlapping_shared_node() {
        let tmp = tempdir().unwrap();
        // A -> B -> A  and  A -> C -> A share node A.
        make_store_with_ticket(tmp.path(), "task-0001", &["task-0002", "task-0003"]);
        make_store_with_ticket(tmp.path(), "task-0002", &["task-0001"]);
        make_store_with_ticket(tmp.path(), "task-0003", &["task-0001"]);

        let (output, has_cycles) = dep_cycle_impl(Some(tmp.path())).unwrap();
        assert!(has_cycles, "expected cycles; output:\n{output}");
        assert_eq!(
            output.matches("Cycle ").count(),
            2,
            "expected two cycles (both A-B-A and A-C-A); output:\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // dep cycle — output format
    // -----------------------------------------------------------------------

    #[test]
    fn cycle_output_format() {
        let tmp = tempdir().unwrap();
        make_store_with_ticket_status(
            tmp.path(),
            "task-0001",
            "First task",
            "open",
            &["task-0002"],
        );
        make_store_with_ticket_status(
            tmp.path(),
            "task-0002",
            "Second task",
            "open",
            &["task-0001"],
        );

        let (output, has_cycles) = dep_cycle_impl(Some(tmp.path())).unwrap();
        assert!(has_cycles, "expected a cycle; output:\n{output}");

        // The chain line must use " -> " arrows.
        assert!(
            output.contains(" -> "),
            "expected arrow notation in chain; output:\n{output}"
        );
        // The chain must end with the start node (closing the loop).
        let first_line = output.lines().next().unwrap_or("");
        assert!(
            first_line.starts_with("Cycle 1: "),
            "first line should start with 'Cycle 1: '; output:\n{output}"
        );
        // Member lines must show [status] and title.
        assert!(
            output.contains("[open]"),
            "expected [open] status in member lines; output:\n{output}"
        );
        assert!(
            output.contains("First task") || output.contains("Second task"),
            "expected ticket title in member lines; output:\n{output}"
        );
    }
}
