---
id: tr-fbj8
status: closed
deps: [tr-kspr]
links: []
created: 2026-03-08T06:31:23Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-h2xw
tags: [phase-3, command]
---
# Implement ls/list command

Create src/commands/list.rs. Implement ls (aliased as list) to display all tickets. Output format: 'ID       [STATUS] - TITLE <- [DEPS]' (deps only shown if non-empty). Support filters: --status=STATUS (open, in_progress, closed), -a/--assignee NAME (match assignee field), -T/--tag TAG (match any tag in tags array). Sort by priority then ID. Read all tickets from store, apply filters, format output.

## Testing

Write unit tests in a `#[cfg(test)]` module in `src/commands/list.rs`. Build in-memory `Vec<Ticket>` collections to test filtering and formatting logic without filesystem access.

- **Lists all tickets**: pass two tickets with no filters; assert both IDs and titles appear in the output.
- **Output format**: assert a single ticket's line matches `"ID  [STATUS] - TITLE"`.
- **Deps shown when non-empty**: a ticket with `deps: [dep-001]`; assert `"<- [dep-001]"` appears on the line.
- **Deps hidden when empty**: a ticket with `deps: []`; assert `"<-"` does not appear on the line.
- **`--status` filter — keeps matching**: filter by `open`; assert only open tickets appear.
- **`--status` filter — excludes non-matching**: assert closed tickets are excluded when filtering for `open`.
- **`-a`/`--assignee` filter**: filter by `"Alice"`; assert only Alice's tickets appear.
- **`-T`/`--tag` filter**: filter by `"backend"`; assert only tickets with that tag appear.
- **Sort by priority then ID**: tickets with priorities 3, 1, 1 and IDs `c`, `b`, `a`; assert output order is priority-1/ID-a, priority-1/ID-b, priority-3/ID-c.
- **Empty list**: pass no tickets; assert output is empty.
- **`list` alias works**: the same handler function is callable via both `ls` and `list` subcommands (test via CLI dispatch or by calling the handler directly with both names).

## BDD Integration Tests

`ls`/`list` scenarios live in the listing feature alongside `ready`, `blocked`, and `closed`. Once `ls` is wired up, run:

```bash
TICKET_SCRIPT=./target/debug/ticket behave features/ticket_listing.feature
```

The scenarios for `ls` cover output format, status/assignee/tag filters, and sort order. The `ready`, `blocked`, and `closed` scenarios in the same file will fail until those commands are also implemented (tr-gkxo, tr-eqrh, tr-kpds) — that's expected and acceptable while working incrementally.
