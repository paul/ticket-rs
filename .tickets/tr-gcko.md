---
id: tr-gcko
status: open
deps: [tr-hs04]
links: []
created: 2026-03-08T06:32:37Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-09dq
tags: [phase-6, plugins]
---
# Implement super command

Add super <cmd> [args] subcommand. Bypass plugin discovery and run the built-in command directly. This is used by plugins that want to call the original built-in (e.g., a show plugin that wraps super show). In the dispatch logic in main.rs, super should skip the plugin check and go straight to built-in command matching.

## BDD Integration Tests

```bash
TICKET_SCRIPT=./target/debug/ticket behave features/ticket_plugins.feature
```

The plugin feature includes a scenario where a plugin calls `$TK_SCRIPT super create "$@"` to delegate to the built-in create command. This is the primary BDD coverage for `super`. Run after tr-hs04 is complete, as plugin dispatch is a prerequisite.

