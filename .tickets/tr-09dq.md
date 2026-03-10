---
id: tr-09dq
status: closed
deps: [tr-xevn, tr-h2xw, tr-r6um, tr-13ck]
links: []
created: 2026-03-08T06:29:13Z
type: epic
priority: 1
assignee: Paul Sadauskas
parent: tr-ketw
tags: [phase-6, plugins, polish]
---
# Phase 6: Plugin system and polish

Implement external plugin discovery and dispatch (ticket-xyz and tk-xyz in PATH), super command (bypass plugins), plugin descriptions in help output. Add syntax highlighting via syntect for show output (YAML frontmatter + Markdown body). Respect NO_COLOR env var, TTY detection, --color=auto|always|never flag. Pager support (TICKET_PAGER or PAGER). Integration tests for all commands.

