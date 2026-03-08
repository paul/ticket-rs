---
id: tr-ketw
status: open
deps: []
links: []
created: 2026-03-08T06:28:39Z
type: epic
priority: 1
assignee: Paul Sadauskas
---
# Port ticket CLI to Rust

Port the bash ticket CLI to a Rust implementation. Same markdown+YAML ticket format, same commands, same external plugin contract. Binary name: ticket. Single crate with clean lib.rs/main.rs separation (workspace later if needed). Dependencies: clap (derive), serde + serde_yaml + serde_json, chrono, rand, syntect (default-fancy).

