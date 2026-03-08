---
id: tr-pybc
status: open
deps: [tr-aaqe]
links: []
created: 2026-03-08T06:30:58Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-xevn
tags: [phase-2, command]
---
# Implement dep tree command

Add dep tree [--full] <id> subcommand. Walk the dependency graph recursively from the given ticket. Display as ASCII tree using box-drawing chars (├──, └──, │). Each node shows ticket ID, [status], and title. Default behavior: deduplicate nodes (show each ticket once at its deepest nesting). --full flag disables dedup for verbose view. Detect and annotate circular dependencies (don't infinite loop). Support partial ID for the root ticket.

