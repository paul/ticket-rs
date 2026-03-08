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
