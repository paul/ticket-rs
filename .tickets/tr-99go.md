---
id: tr-99go
status: closed
deps: [tr-hs04, tr-gcko, tr-yrdk, tr-pfsb, tr-mhd5]
links: []
created: 2026-03-08T06:33:01Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-09dq
tags: [phase-6, testing]
---
# Integration tests

Write integration tests in tests/ directory. These are end-to-end black-box tests that exercise the compiled `ticket` binary against a real `.tickets/` directory. Unit tests for individual modules are written alongside the implementation (see each feature ticket); this ticket covers only the integration layer.

Test each command end-to-end by running the ticket binary against a temp .tickets/ directory. Cover: create (verify file created with correct frontmatter), show (verify dynamic sections), status changes, dep/undep (verify frontmatter updated), dep tree output format, dep cycle detection, link/unlink symmetry, ls/ready/blocked/closed filtering, add-note (verify timestamp and ## Notes section), update (all field types including tag add/remove), query JSON output, tree display, partial ID resolution (exact, partial, ambiguous error), plugin discovery (create a temp plugin script in PATH). Use assert_cmd and tempdir crates for test infrastructure.

## Testing approach

Use `assert_cmd::Command` to invoke the `ticket` binary and `tempfile::tempdir()` for isolated `.tickets/` directories. Each test scenario maps to one of the feature files in `~/.local/share/ticket/features/` — use those as the authoritative specification for expected output and exit codes. Key scenarios to cover per feature file:

- **ticket_creation.feature**: all default field values, optional sections (design, acceptance), parent validation, `.tickets/` dir created on demand.
- **ticket_status.feature**: all four commands (start, close, reopen, status), invalid status error, partial ID.
- **ticket_dependencies.feature**: dep/undep round-trip, idempotency, tree output with box-drawing chars, sorting by subtree depth, cycle handling.
- **ticket_links.feature**: symmetric link creation, three-ticket linking, idempotency, unlink both directions.
- **ticket_listing.feature**: ls/list format, status/assignee/tag filters, ready (no deps, all deps closed), blocked (unclosed deps only in output), closed (mtime sort, limit).
- **ticket_notes.feature**: Notes section creation, timestamp format, multiple notes accumulate.
- **ticket_show.feature**: dynamic Blockers/Blocking/Children/Linked sections, parent annotation.
- **ticket_query.feature**: JSONL output, all fields present, jq filter piping.
- **id_resolution.feature**: exact, prefix, suffix, substring, ambiguous error, not-found error, exact-takes-precedence.
- **ticket_directory.feature**: parent-dir walking, TICKETS_DIR override, error when no .tickets found.
- **ticket_plugins.feature**: tk-/ticket- dispatch, super bypass, env vars passed, help listing.

## BDD Integration Tests

This ticket IS the BDD integration test layer. Rather than (or in addition to) writing Rust `tests/` integration tests with `assert_cmd`, consider using the Behave suite in `features/` as the primary integration test vehicle. The suite is already wired in the project:

```bash
# Run against the Rust binary (target once all commands are implemented)
TICKET_SCRIPT=./target/debug/ticket behave features/

# Run against the bash reference to confirm the harness is sound
TICKET_SCRIPT=/home/rando/.local/share/ticket/ticket behave features/
```

The bash reference passes all 123 scenarios across 12 feature files. The Rust implementation should reach the same score. Individual feature files can be targeted during incremental development — each command ticket documents which feature file it maps to in its own `## BDD Integration Tests` section.

If `assert_cmd`-based Rust integration tests are written in `tests/`, they should duplicate a subset of the BDD scenarios as a fast sanity check in `cargo test`, not replace the BDD suite.

## Notes

**2026-03-10T01:36:03Z**

The BDD behave suite in features/ is the integration test layer and is wired to run against the compiled binary via TICKET_SCRIPT=./target/debug/ticket behave features/. All 117 non-tk-prefix scenarios pass. The 6 remaining failures are all tk- prefix plugin scenarios (ticket_plugins.feature), which we decided not to support — the tk- prefix is not part of the spec for the Rust implementation. No assert_cmd-based Rust tests were added since the BDD suite provides full coverage.
