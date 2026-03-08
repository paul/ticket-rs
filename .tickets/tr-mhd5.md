---
id: tr-mhd5
status: open
deps: [tr-hs04]
links: []
created: 2026-03-08T06:32:53Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-09dq
tags: [phase-6, plugins]
---
# Implement help command with plugin listing

Enhance the help output to include discovered plugins alongside built-in commands. Use discover_plugins() from plugin.rs to find all ticket-*/tk-* executables in PATH. Display built-in commands first (from clap's built-in help), then a 'Plugins' section listing each discovered plugin with its description. Match the bash version's help format. Exclude built-in command names from plugin listing (don't double-list if a plugin overrides a built-in).

