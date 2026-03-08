---
id: tr-pfsb
status: open
deps: [tr-fz7v]
links: []
created: 2026-03-08T06:32:48Z
type: task
priority: 3
assignee: Paul Sadauskas
parent: tr-09dq
tags: [phase-6, polish]
---
# Implement pager support

Add pager support to show command output. Check TICKET_PAGER env var first, then PAGER, then fall back to no pager. Only page when stdout is a TTY — use `console::Term::stdout().is_term()` for TTY detection rather than rolling it manually. Pipe the full show output through the pager command via a child process. Handle pager exit gracefully (broken pipe is not an error). Shell out to the pager rather than using a pager crate.

