---
id: tr-eqrh
status: open
deps: [tr-kspr]
links: []
created: 2026-03-08T06:31:32Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-h2xw
tags: [phase-3, command]
---
# Implement blocked command

Add blocked subcommand to src/commands/list.rs. Show open/in_progress tickets where at least one dependency is NOT closed. Sort by priority then ID. Output format: 'ID       [PN][STATUS] - TITLE <- [OPEN_DEPS]' showing only the unclosed blocking deps. Support -a/--assignee and -T/--tag filters.

## Testing

Write unit tests in a `#[cfg(test)]` module in `src/commands/list.rs`. Use in-memory `Vec<Ticket>` collections.

- **Ticket with any unclosed dep is blocked**: a ticket depending on an open ticket; assert it appears in blocked output.
- **Ticket with all deps closed is not blocked**: assert it is excluded from blocked output.
- **Ticket with no deps is not blocked**: assert it is excluded.
- **Closed ticket is not blocked**: a closed ticket with an open dep; assert it is excluded.
- **Only unclosed deps shown in output**: a ticket with one open dep and one closed dep; assert only the open dep appears in the `<- [...]` list.
- **Priority badge format**: assert output line matches `"ID  [P2][open] - TITLE <- [blocker-id]"`.
- **Sort by priority then ID**: verify correct ordering across multiple blocked tickets.
- **`-a` assignee filter**: only show blocked tickets for the specified assignee.
- **`-T` tag filter**: only show blocked tickets with the specified tag.
- **Empty output**: when no tickets are blocked, assert output is empty.
