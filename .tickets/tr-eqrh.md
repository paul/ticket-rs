---
id: tr-eqrh
status: open
deps: [tr-kspr]
links: []
created: 2026-03-08T06:31:32Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-h2xw
tags: [phase-3, command]
---
# Implement blocked command

Add blocked subcommand to src/commands/list.rs. Show open/in_progress tickets where at least one dependency is NOT closed. Sort by priority then ID. Output format: 'ID       [PN][STATUS] - TITLE <- [OPEN_DEPS]' showing only the unclosed blocking deps. Support -a/--assignee and -T/--tag filters.

