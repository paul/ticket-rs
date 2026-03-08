---
id: tr-gkxo
status: open
deps: [tr-kspr]
links: []
created: 2026-03-08T06:31:28Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-h2xw
tags: [phase-3, command]
---
# Implement ready command

Add ready subcommand to src/commands/list.rs. Show open/in_progress tickets where ALL dependencies are closed (or deps list is empty). Sort by priority (ascending, 0=highest) then ID. Display priority badge: [P0], [P1], etc. Output format: 'ID       [PN][STATUS] - TITLE'. Support -a/--assignee and -T/--tag filters. Must load all tickets to check dep statuses.

