---
id: tr-2ns4
status: open
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

