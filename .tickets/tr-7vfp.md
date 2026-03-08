---
id: tr-7vfp
status: open
deps: []
links: []
created: 2026-03-08T06:29:43Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-3kr6
tags: [phase-1, core]
---
# Implement ID generation

Create src/id.rs. Generate ticket IDs as PREFIX-SUFFIX where: PREFIX is 2-4 chars derived from directory name (multi-segment names like my-project use first letter of each segment -> mp; single-segment names use first 3 chars -> pla). SUFFIX is 4 random lowercase hex chars from rand crate. Provide generate_id(dir_name: &str) -> String function. Must match the bash version's prefix algorithm exactly for format compatibility.

