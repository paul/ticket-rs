---
id: tr-siyb
status: open
deps: [tr-6u6p]
links: []
created: 2026-03-08T06:30:08Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-3kr6
tags: [phase-1, cli]
---
# Implement clap CLI skeleton

Create src/cli.rs with clap derive-based command definitions. Define the top-level Cli struct and Commands enum with all subcommands stubbed out. Start with functional definitions for Phase 1 commands (create, show, start, close, reopen, status). Other phases' commands can be defined as stubs that print 'not yet implemented'. Include global --color=auto|always|never flag. The binary should dispatch to command handler functions in src/commands/.

