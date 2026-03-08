---
id: tr-fbj8
status: open
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

