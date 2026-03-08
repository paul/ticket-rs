---
id: tr-fz7v
status: closed
deps: [tr-siyb, tr-kspr]
links: []
created: 2026-03-08T06:30:25Z
type: task
priority: 1
assignee: Paul Sadauskas
parent: tr-3kr6
tags: [phase-1, command]
---
# Implement show command

Create src/commands/show.rs. Resolve partial ID, read ticket file, display full markdown content. Append dynamic sections computed at display time: ## Blockers (unclosed deps with [status] and title), ## Blocking (tickets that list this one in their deps), ## Children (tickets with this as parent), ## Linked (tickets in this ticket's links array, with [status] and title). If parent field is set, show parent title as inline annotation. Output through pager if stdout is TTY (TICKET_PAGER or PAGER env var). Syntax highlighting via syntect is deferred to Phase 6 but the command should work without it.

## Testing

Write unit tests in a `#[cfg(test)]` module in `src/commands/show.rs`, testing the output-building logic independently from the pager/TTY layer. Use `tempfile::tempdir()` for filesystem tests.

- **Displays raw content**: call the show output builder with a simple ticket; assert the frontmatter and `# Title` appear in the output.
- **All frontmatter fields shown**: assert `status:`, `deps:`, `links:`, `type:`, `priority:` all appear in output.
- **`## Blockers` section — present when deps unclosed**: create a ticket with one open dep; assert the output contains `## Blockers` and the dep's ID and title.
- **`## Blockers` section — absent when all deps closed**: close all deps; assert `## Blockers` does not appear in output.
- **`## Blocking` section**: create ticket B that depends on ticket A; show ticket A and assert `## Blocking` contains ticket B.
- **`## Children` section**: create a ticket with `parent` pointing to another; show the parent and assert `## Children` contains the child.
- **`## Linked` section**: create two linked tickets; show one and assert `## Linked` contains the other with `[status]` and title.
- **Parent annotation**: show a ticket with a `parent` field set; assert the parent's title appears in the output (as an inline annotation or comment after the parent ID).
- **Non-existent ticket**: assert a `TicketNotFound` error.
- **Partial ID resolution**: create ticket `show-001`, show via `"001"`, assert the output contains `id: show-001`.

## BDD Integration Tests

Once `show` is wired into the binary, run:

```bash
TICKET_SCRIPT=./target/debug/ticket behave features/ticket_show.feature
```

The scenarios cover: displaying frontmatter and title, the dynamic `## Blockers` / `## Blocking` / `## Children` / `## Linked` sections, parent annotation, and non-existent ticket errors. Run with `--no-capture` to see raw output on failure. The pager is not exercised by the BDD suite (tests run non-TTY), so pager integration is tested separately in tr-pfsb.
