---
id: tr-n2ln
status: open
deps: []
links: []
created: 2026-03-08T06:29:36Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-3kr6
tags: [phase-1, core]
---
# Implement error types

Create src/error.rs with a unified error enum covering: ticket not found, ambiguous ID match (multiple matches), tickets directory not found, YAML parse errors, IO errors, invalid status/type/priority values. Implement Display and std::error::Error. Consider using thiserror crate or manual impl.

