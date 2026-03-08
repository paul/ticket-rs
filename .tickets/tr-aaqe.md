---
id: tr-aaqe
status: open
deps: [tr-kspr]
links: []
created: 2026-03-08T06:30:51Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-xevn
tags: [phase-2, command]
---
# Implement dep and undep commands

Create src/commands/dep.rs. Implement dep <id> <dep-id>: add dep-id to the deps array of ticket id. Resolve both IDs via partial matching. Prevent duplicates. Implement undep <id> <dep-id>: remove dep-id from the deps array. Validate the dependency exists before removing. Both commands update the YAML frontmatter deps field and write back.

## Testing

Write unit tests in a `#[cfg(test)]` module in `src/commands/dep.rs`. Use `tempfile::tempdir()` for filesystem tests.

- **`dep` adds dependency**: write two tickets; call `dep A B`; read ticket A back and assert `B` is in its `deps` array.
- **`dep` is idempotent**: call `dep A B` twice; assert `B` appears exactly once in `A`'s `deps`.
- **`dep` output message**: assert stdout is `"Added dependency: A -> B"`.
- **`dep` — already exists output**: when the dep already exists, assert stdout is `"Dependency already exists"`.
- **`undep` removes dependency**: set up `A` depending on `B`; call `undep A B`; assert `B` is no longer in `A`'s `deps`.
- **`undep` output message**: assert stdout is `"Removed dependency: A -/-> B"`.
- **`undep` — not found error**: call `undep A B` when no such dep exists; assert failure with `"Dependency not found"`.
- **`dep` — non-existent dep ticket**: call `dep A nonexistent`; assert a `TicketNotFound` error.
- **`dep` — non-existent source ticket**: call `dep nonexistent B`; assert a `TicketNotFound` error.
- **Partial ID resolution**: use partial IDs for both arguments; assert the correct tickets are updated.
- **Frontmatter preserved**: after `dep`, assert all other frontmatter fields and the body are unchanged.

## BDD Integration Tests

```bash
TICKET_SCRIPT=./target/debug/ticket behave features/ticket_dependencies.feature
```

The `dep`/`undep` scenarios in `ticket_dependencies.feature` cover: adding a dependency, idempotency, removing a dependency, removing non-existent dependency, and errors for non-existent tickets. The `dep tree` and `dep cycle` scenarios in the same file will fail until tr-pybc and tr-23te are implemented — that's expected while working incrementally.
