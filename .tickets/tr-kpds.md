---
id: tr-kpds
status: open
deps: [tr-kspr]
links: []
created: 2026-03-08T06:31:36Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-h2xw
tags: [phase-3, command]
---
# Implement closed command

Add closed subcommand to src/commands/list.rs. Show recently closed tickets sorted by file modification time (most recent first). --limit=N flag (default 20). For efficiency, only scan the N most recently modified files rather than loading all tickets. Output format: 'ID       [closed] - TITLE'. Support -a/--assignee and -T/--tag filters.

