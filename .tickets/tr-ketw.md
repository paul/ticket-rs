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

## Testing

Unit tests are written alongside each module as implementation progresses — not deferred to a separate phase. Each source file that contains non-trivial logic must have a `#[cfg(test)]` module. End-to-end integration tests (using `assert_cmd` and `tempfile`) are tracked separately in tr-99go and are written after all commands are implemented. Add `tempfile` as a dev-dependency in Cargo.toml from the start to unblock unit testing of filesystem operations.
