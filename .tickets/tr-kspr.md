---
id: tr-kspr
status: closed
deps: [tr-9thi, tr-7vfp, tr-n2ln]
links: []
created: 2026-03-08T06:29:58Z
type: task
priority: 1
assignee: Paul Sadauskas
parent: tr-3kr6
tags: [phase-1, core]
---
# Implement ticket store (directory operations)

Create src/store.rs. Implement: find_tickets_dir() that walks parent directories looking for .tickets/ (check current dir, then parent, grandparent, etc.; respect TICKETS_DIR env var override). read_ticket(id) -> Result<Ticket> to read and parse a ticket file. write_ticket(ticket) -> Result<()> to write a ticket to disk. list_tickets() -> Vec<Ticket> to read all .md files in .tickets/. resolve_id(partial: &str) -> Result<PathBuf> for partial ID matching (exact match takes precedence, partial must be unique, error on ambiguous). ensure_dir() to create .tickets/ if needed. The store should be a struct that holds the resolved tickets directory path.

## Testing

Write unit tests in a `#[cfg(test)]` module at the bottom of `src/store.rs`. Use `tempfile::tempdir()` for all tests that touch the filesystem.

- **`find_tickets_dir` — current directory**: create a `.tickets/` dir in a temp dir, call `find_tickets_dir` from that directory, assert the correct path is returned.
- **`find_tickets_dir` — parent directory**: create `.tickets/` in a temp root, call from a nested subdirectory (`src/components`), assert the parent's `.tickets/` is found.
- **`find_tickets_dir` — grandparent directory**: repeat with two levels of nesting to confirm the full ancestor walk.
- **`find_tickets_dir` — `TICKETS_DIR` env override**: set `TICKETS_DIR` to an arbitrary path and assert it is returned without walking parents. Restore the env var after the test.
- **`find_tickets_dir` — not found**: assert a `TicketsNotFound` error is returned when no `.tickets/` exists in any ancestor.
- **`resolve_id` — exact match**: write a ticket file `abc-1234.md`; assert `resolve_id("abc-1234")` returns its path.
- **`resolve_id` — prefix match**: assert `resolve_id("abc")` resolves to `abc-1234.md`.
- **`resolve_id` — suffix match**: assert `resolve_id("1234")` resolves to `abc-1234.md`.
- **`resolve_id` — substring match**: assert `resolve_id("c-12")` resolves to `abc-1234.md`.
- **`resolve_id` — exact takes precedence over partial**: write both `abc.md` and `abc-1234.md`; assert `resolve_id("abc")` returns `abc.md`, not an ambiguity error.
- **`resolve_id` — ambiguous error**: write `abc-1234.md` and `abc-5678.md`; assert `resolve_id("abc")` returns an `AmbiguousId` error listing both matches.
- **`resolve_id` — not found error**: assert `resolve_id("nonexistent")` returns a `TicketNotFound` error.
- **`read_ticket` / `write_ticket` round-trip**: write a `Ticket` to disk with `write_ticket`, read it back with `read_ticket`, assert all fields are equal to the original.
- **`list_tickets`**: write three ticket files to a temp `.tickets/` dir, call `list_tickets`, assert all three are returned.
- **`ensure_dir`**: call on a temp dir without a `.tickets/` subdir; assert the directory is subsequently present.

## BDD Integration Tests

The store underpins all commands, but two feature files exercise its behavior directly. Once any commands that depend on the store are wired up, run:

```bash
# Partial ID resolution (exact, prefix, suffix, substring, ambiguous, exact-takes-precedence)
TICKET_SCRIPT=./target/debug/ticket behave features/id_resolution.feature

# Directory walking, TICKETS_DIR env override, error when no .tickets found
TICKET_SCRIPT=./target/debug/ticket behave features/ticket_directory.feature
```

`id_resolution.feature` requires `show`, `status`, `dep`, and `link` to be functional. `ticket_directory.feature` requires `ls`, `show`, `create`, `dep`, and `ready`. Run these suites after tr-04qd, tr-fz7v, tr-2ns4, tr-fbj8, and tr-gkxo are complete.
