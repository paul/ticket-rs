---
id: tr-6u6p
status: open
deps: []
links: []
created: 2026-03-08T06:29:32Z
type: task
priority: 1
assignee: Paul Sadauskas
parent: tr-3kr6
tags: [phase-1, setup]
---
# Set up Cargo project and dependencies

Fix Cargo.toml edition (currently 2024, should be 2021). Add dependencies: clap (derive features), serde + serde_yaml + serde_json, chrono (serde feature), rand, syntect (default-fancy). Set up src/main.rs, src/lib.rs, and module structure: cli.rs, ticket.rs, store.rs, id.rs, error.rs, highlight.rs, plugin.rs, commands/mod.rs. Binary name: ticket.

