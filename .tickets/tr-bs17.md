---
id: tr-bs17
status: open
deps: [tr-kspr]
links: []
created: 2026-03-08T06:31:08Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-xevn
tags: [phase-2, command]
---
# Implement link and unlink commands

Add to src/commands/link.rs. Implement link <id> <id> [id...]: create symmetric links between all specified tickets. For each pair, add each ID to the other's links array. Prevent duplicates. Support 2+ tickets in one call. Implement unlink <id> <target-id>: remove symmetric link between two tickets (remove from both tickets' links arrays). Validate link exists before removing. All IDs resolved via partial matching.

## Testing

Write unit tests in a `#[cfg(test)]` module in `src/commands/link.rs`. Use `tempfile::tempdir()` for filesystem tests.

- **`link` two tickets — symmetric**: call `link A B`; assert `B` is in `A`'s `links` and `A` is in `B`'s `links`.
- **`link` three tickets — all pairs**: call `link A B C`; assert every ticket has the other two in its `links`.
- **`link` output — two tickets**: assert stdout is `"Added 2 link(s) between 2 tickets"`.
- **`link` output — three tickets**: assert stdout is `"Added 6 link(s) between 3 tickets"`.
- **`link` is idempotent**: call `link A B` when already linked; assert stdout is `"All links already exist"` and each ID still appears exactly once in the other's `links`.
- **`link` partial — only new pairs added**: pre-link A↔B, then call `link A B C`; assert only the A↔C and B↔C links are new, output reports the correct count.
- **`unlink` removes both directions**: set up A↔B link; call `unlink A B`; assert neither ticket has the other in its `links`.
- **`unlink` output**: assert stdout is `"Removed link: A <-> B"`.
- **`unlink` — link not found**: call `unlink A B` when no link exists; assert failure with `"Link not found"`.
- **Non-existent ticket**: call `link A nonexistent`; assert a `TicketNotFound` error.
- **Partial ID resolution**: use partial IDs; assert the correct tickets are updated.

## BDD Integration Tests

```bash
TICKET_SCRIPT=./target/debug/ticket behave features/ticket_links.feature
```

Scenarios cover: linking two tickets symmetrically, linking three tickets (all pairs), idempotency, partial-update when some links already exist, unlinking both directions, unlink-not-found error, non-existent ticket error, and output message format. All scenarios must pass — the link count in output messages (e.g. `"Added 6 link(s) between 3 tickets"`) must match exactly.
