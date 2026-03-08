---
id: tr-hs04
status: open
deps: [tr-siyb]
links: []
created: 2026-03-08T06:32:32Z
type: task
priority: 1
assignee: Paul Sadauskas
parent: tr-09dq
tags: [phase-6, plugins]
---
# Implement external plugin discovery and dispatch

Create src/plugin.rs. When a command is not a built-in, search PATH for executables named ticket-<cmd> then tk-<cmd>. If found, exec the plugin with remaining args, passing TICKETS_DIR and TK_SCRIPT (path to own binary) as env vars. Implement discover_plugins() -> Vec<PluginInfo> that scans PATH for all ticket-* and tk-* executables, extracts descriptions from '# tk-plugin:' comment in first 10 lines (for scripts) or --tk-describe flag output (for binaries). Used by help command to list available plugins.

