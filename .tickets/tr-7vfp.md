---
id: tr-7vfp
status: closed
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

## Testing

Write unit tests in a `#[cfg(test)]` module at the bottom of `src/id.rs`. Test the prefix derivation logic directly (extract it into a pure `derive_prefix(dir_name: &str) -> String` helper) so tests don't need to account for randomness.

- **Multi-segment name**: `"my-project"` → prefix `"mp"`.
- **Three-segment name**: `"ticket-cli-rs"` → prefix `"tcr"`.
- **Four-segment name**: `"my-big-rust-app"` → prefix `"mbra"`.
- **Single-segment short name** (≤3 chars): `"tk"` → prefix `"tk"`.
- **Single-segment long name**: `"platform"` → prefix `"pla"`.
- **Single-segment exactly 3 chars**: `"foo"` → prefix `"foo"`.
- **`generate_id` format**: call `generate_id` with a known dir name and assert the result matches the regex `^[a-z]{2,4}-[0-9a-f]{4}$`.
- **`generate_id` suffix is hex**: assert the suffix portion contains only characters in `[0-9a-f]`.
- **`generate_id` suffix length**: assert the suffix is exactly 4 characters.

## Notes

During review, two issues were found and fixed before approval:

1. Suffix validation in tests used `is_ascii_hexdigit()` which accepts uppercase `A-F`, violating the lowercase-only `[0-9a-f]` requirement. Fixed by replacing with `matches!(c, '0'..='9' | 'a'..='f')` in both `generate_id_suffix_is_hex` and the `regex_lite` helper.
2. Clippy warned on `.split('-').last()` over a `DoubleEndedIterator` (needlessly iterates the whole iterator). Fixed by switching to `.split('-').next_back()`.

As a follow-on from the review, the Behave/BDD integration test suite from the bash ticket implementation was copied into `features/` and confirmed to run cleanly (123 scenarios, 0 failures) against the bash `ticket` script via `TICKET_SCRIPT=/path/to/ticket behave features/`. This provides a regression baseline; as Rust commands are implemented the same suite can be run with `TICKET_SCRIPT=./target/debug/ticket` to track parity.
