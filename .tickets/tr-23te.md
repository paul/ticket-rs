---
id: tr-23te
status: open
deps: [tr-aaqe]
links: []
created: 2026-03-08T06:31:03Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-xevn
tags: [phase-2, command]
---
# Implement dep cycle command

Add dep cycle subcommand. Run DFS-based cycle detection across all open/in_progress tickets. Only consider open and in_progress tickets (skip closed). Normalize and deduplicate detected cycles. Output each cycle showing the chain (a -> b -> c -> a) and listing each ticket with [status] and title. Exit 0 if no cycles found, exit 1 if cycles detected.

