---
id: tr-gkxo
status: open
deps: [tr-kspr]
links: []
created: 2026-03-08T06:31:28Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-h2xw
tags: [phase-3, command]
---
# Implement ready command

Add ready subcommand to src/commands/list.rs. Show open/in_progress tickets where ALL dependencies are closed (or deps list is empty). Sort by priority (ascending, 0=highest) then ID. Display priority badge: [P0], [P1], etc. Output format: 'ID       [PN][STATUS] - TITLE'. Support -a/--assignee and -T/--tag filters. Must load all tickets to check dep statuses.

## Testing

Write unit tests in a `#[cfg(test)]` module in `src/commands/list.rs`. Use in-memory `Vec<Ticket>` collections.

- **Ticket with no deps is ready**: a ticket with `deps: []`; assert it appears in ready output.
- **Ticket with all deps closed is ready**: a ticket depending on a closed ticket; assert it appears.
- **Ticket with any unclosed dep is not ready**: a ticket depending on an open ticket; assert it is excluded.
- **Closed ticket is not ready**: a closed ticket with no deps; assert it is excluded.
- **`in_progress` ticket with all deps closed is ready**: assert `in_progress` tickets are included when eligible.
- **Priority badge format**: assert output line matches `"ID  [P2][open] - TITLE"`.
- **Sort by priority then ID**: tickets at priority 1/ID `b` and priority 1/ID `a` and priority 3/ID `c`; assert order is `a`, `b`, `c`.
- **`-a` assignee filter**: only show ready tickets assigned to the specified user.
- **`-T` tag filter**: only show ready tickets with the specified tag.
- **Empty output**: when no tickets are ready, assert output is empty.

## BDD Integration Tests

```bash
TICKET_SCRIPT=./target/debug/ticket behave features/ticket_listing.feature
```

The `ready` scenarios in `ticket_listing.feature` cover: tickets with no deps, all deps closed, any unclosed dep, closed tickets excluded, priority badge format, sort order, and assignee/tag filters. Run the full listing feature file after tr-fbj8, tr-eqrh, and tr-kpds are all complete to validate the whole listing surface together.
