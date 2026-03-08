---
id: tr-2ns4
status: closed
deps: [tr-siyb, tr-kspr]
links: []
created: 2026-03-08T06:30:31Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-3kr6
tags: [phase-1, command]
---
# Implement status commands (start, close, reopen, status)

Create src/commands/status.rs. Implement four entry points: start <id> (set status to in_progress), close <id> (set status to closed), reopen <id> (set status to open), status <id> <status> (explicit set, validate status is one of open/in_progress/closed). All resolve partial IDs. Read the ticket file, update the status field in YAML frontmatter, write back. Must preserve the rest of the file content exactly (don't re-serialize the whole file, just update the status field).

## Testing

Write unit tests in a `#[cfg(test)]` module in `src/commands/status.rs`. Use `tempfile::tempdir()` for filesystem tests.

- **`start` sets `in_progress`**: write an open ticket, call `start`, read back and assert `status: in_progress`.
- **`close` sets `closed`**: write an open ticket, call `close`, read back and assert `status: closed`.
- **`reopen` sets `open`**: write a closed ticket, call `reopen`, read back and assert `status: open`.
- **`status` — explicit `in_progress`**: call `status <id> in_progress`, assert frontmatter updated.
- **`status` — explicit `closed`**: call `status <id> closed`, assert frontmatter updated.
- **`status` — explicit `open`**: call `status <id> open`, assert frontmatter updated.
- **Invalid status value**: call `status <id> invalid`; assert the command returns an `InvalidStatus` error whose message names `"invalid"` and lists the valid options.
- **File content preserved**: write a ticket file with body text and a `## Notes` section; run `close`; assert all body content after the frontmatter is byte-identical to the original (only the `status:` line should change).
- **Partial ID resolution**: write a ticket with ID `test-9999`, call `start "9999"`, assert the correct file is updated.
- **Non-existent ticket**: assert a `TicketNotFound` error for an unknown ID.
- **Output message**: assert stdout contains `"Updated <id> -> <new_status>"`.

## BDD Integration Tests

Once `start`, `close`, `reopen`, and `status` are wired into the binary, run:

```bash
TICKET_SCRIPT=./target/debug/ticket behave features/ticket_status.feature
```

Scenarios cover all four commands, invalid status rejection, output messages, and partial ID resolution. The feature file is the authoritative output-format spec — match it exactly.

## Notes

**2026-03-08T21:57:04Z**

Implementation used the existing Ticket::read_from_str / write_to_string round-trip (already proven byte-identical in ticket.rs tests) rather than string-bashing the status field. set_status_impl returns the full output message string so each public function does println!("{msg}") with no independent format call — eliminating the typo risk. InvalidStatus display was changed to space-separate valid options (open in_progress closed) to match the BDD feature file assertion. All 86 unit tests pass; all 9 BDD scenarios pass.
