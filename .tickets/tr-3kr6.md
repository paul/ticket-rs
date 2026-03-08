---
id: tr-3kr6
status: open
deps: []
links: []
created: 2026-03-08T06:28:47Z
type: epic
priority: 1
assignee: Paul Sadauskas
parent: tr-ketw
tags: [phase-1, foundation]
---
# Phase 1: Project setup and core library

Set up the Rust project structure and implement the core libticket modules: ticket.rs (Ticket struct, YAML frontmatter serde, markdown parsing/writing), store.rs (.tickets/ directory operations, file read/write/list, partial ID resolution, directory walking up parents), id.rs (prefix-from-dirname + 4-char random hex suffix), error.rs (error types). Also implement the CLI skeleton with clap and the first basic commands: create, show, status/start/close/reopen.

