---
id: tr-b3ex
status: open
deps: [tr-kspr]
links: []
created: 2026-03-08T06:32:21Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-13ck
tags: [phase-5, command]
---
# Implement tree command

Create src/commands/tree.rs. Implement tree [ticket-id] [options]. Display parent/child hierarchy (walks the parent field, NOT deps). Options: --max-depth/-L N (limit depth, 0 = root only), --all (include closed tickets, default is open/in_progress only), --no-color. Color by status: in_progress=cyan, open=blue, closed=dim. Sort children by status priority (in_progress < open < closed) then created_at. Detect and annotate cycles. If ticket-id provided, show subtree rooted at that ticket. If omitted, show all root tickets (those with no parent or whose parent is not in the visible set). Format: box-drawing chars (├──, └──, │). Respect NO_COLOR env var and TTY detection.

## Testing

Write unit tests in a `#[cfg(test)]` module in `src/commands/tree.rs`. Test tree-building and rendering with in-memory `Vec<Ticket>` collections; strip ANSI codes before asserting on text content.

- **Root ticket shown**: a single ticket with no parent; assert it appears as a root node.
- **Child indented under parent**: ticket B with `parent: A`; assert B appears indented under A with box-drawing characters.
- **Box-drawing characters**: assert output contains `├──` or `└──` for children and `│` for continuation lines.
- **Multiple roots**: two tickets with no parent; assert both appear as root nodes.
- **`--max-depth 0` shows root only**: assert only the root ticket appears, with no children.
- **`--max-depth 1` shows one level**: assert children appear but grandchildren do not.
- **`--all` includes closed tickets**: by default, closed children are hidden; with `--all`, they appear.
- **Sort by status priority then created_at**: `in_progress` children appear before `open`, `open` before `closed`; within same status, earlier `created_at` appears first.
- **Cycle detection — no infinite loop**: A's parent is B and B's parent is A; assert the command terminates and annotates the cycle.
- **Subtree rooted at given ticket**: call `tree B` where B has a parent A and child C; assert only B and C appear (not A).
- **Omitted ID shows all roots**: call `tree` with no argument; assert all tickets with no visible parent are shown as roots.
- **`NO_COLOR` env var disables color**: set the env var and assert no ANSI escape codes appear in the output.
