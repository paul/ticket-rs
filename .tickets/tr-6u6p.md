---
id: tr-6u6p
status: in_progress
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

Add dependencies: clap (derive features), serde + serde_yaml + serde_json, chrono (serde feature), rand, syntect (default-fancy), console. Set up src/main.rs, src/lib.rs, and module structure: cli.rs, ticket.rs, store.rs, id.rs, error.rs, highlight.rs, plugin.rs, commands/mod.rs. Binary name: ticket.

`console` is the terminal styling library for this project. It provides `style("text").cyan().bold()` for colored output, `Term::stdout().is_term()` for TTY detection, `colors_enabled()` for respecting `NO_COLOR`/`CLICOLOR`, and `measure_text_width()` for unicode-aware column alignment. It replaces any need for manual ANSI escape handling outside of syntect.

