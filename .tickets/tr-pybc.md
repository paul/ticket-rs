---
id: tr-pybc
status: open
deps: [tr-aaqe]
links: []
created: 2026-03-08T06:30:58Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-xevn
tags: [phase-2, command]
---
# Implement dep tree command

Add dep tree [--full] <id> subcommand. Walk the dependency graph recursively from the given ticket. Display as ASCII tree using box-drawing chars (├──, └──, │). Each node shows ticket ID, [status], and title. Default behavior: deduplicate nodes (show each ticket once at its deepest nesting). --full flag disables dedup for verbose view. Detect and annotate circular dependencies (don't infinite loop). Support partial ID for the root ticket.

## Testing

Write unit tests in a `#[cfg(test)]` module in `src/commands/dep.rs` (or a dedicated `dep_tree` module). Test the tree-building and rendering logic with in-memory ticket collections rather than the filesystem where possible.

- **Linear chain**: A → B → C; assert output contains all three IDs and box-drawing characters (`├──` or `└──`).
- **Each node shows status and title**: assert `[open]` and each ticket's title appear next to their IDs.
- **Multiple direct deps**: A → {B, C}; assert both B and C appear in the output.
- **Deduplication (default)**: A → B, A → C, B → C; assert C appears only once in the output.
- **`--full` disables dedup**: same graph; assert C appears twice with `--full`.
- **Cycle detection — no infinite loop**: A → B → A; assert the command terminates and annotates the cycle (e.g. `[cycle]`).
- **Sorting — by subtree depth then ID**: shallow children appear before deep ones; within the same depth, sort alphabetically by ID (see feature file scenarios for exact ordering expectations).
- **Partial ID for root**: create ticket `task-0001`, call `dep tree 0001`; assert the tree is rooted at `task-0001`.
- **Non-existent root ticket**: assert a `TicketNotFound` error.

## BDD Integration Tests

```bash
TICKET_SCRIPT=./target/debug/ticket behave features/ticket_dependencies.feature
```

The `dep tree` scenarios cover: basic tree display with box-drawing characters, status and title in each node, multiple direct deps, deduplication (default), `--full` flag disabling dedup, cycle handling, and sorting by subtree depth then ID. The sorting scenarios are particularly precise — the feature file documents the exact expected ordering and should be treated as the spec. Run the full dependency feature file after tr-aaqe and tr-23te are also complete.
