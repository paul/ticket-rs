---
id: tr-hjv9
status: closed
deps: []
links: []
created: 2026-03-08T07:51:46Z
type: chore
priority: 3
assignee: Paul Sadauskas
tags: [tests, readability]
---
# Convert inline YAML strings in tests to multiline Rust raw strings

Several test functions in src/ticket.rs and src/error.rs use single-line escape sequences (\n) to represent YAML fixture strings. These are difficult to read and verify at a glance. They should be converted to multiline Rust raw strings (r#"..."# or the backslash-continuation style already used in FULL_FIXTURE) so the structure of the YAML is visually apparent.

## Acceptance Criteria

All inline YAML strings in tests that use \n escape sequences are converted to multiline raw string literals. Tests still pass after conversion. The YAML content is unchanged — only the Rust string representation changes.

