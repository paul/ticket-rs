---
id: tr-99go
status: open
deps: [tr-hs04, tr-gcko, tr-yrdk, tr-pfsb, tr-mhd5]
links: []
created: 2026-03-08T06:33:01Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-09dq
tags: [phase-6, testing]
---
# Integration tests

Write integration tests in tests/ directory. Test each command end-to-end by running the ticket binary against a temp .tickets/ directory. Cover: create (verify file created with correct frontmatter), show (verify dynamic sections), status changes, dep/undep (verify frontmatter updated), dep tree output format, dep cycle detection, link/unlink symmetry, ls/ready/blocked/closed filtering, add-note (verify timestamp and ## Notes section), update (all field types including tag add/remove), query JSON output, tree display, partial ID resolution (exact, partial, ambiguous error), plugin discovery (create a temp plugin script in PATH). Use assert_cmd and tempdir crates for test infrastructure.

