---
id: tr-jvtu
status: closed
deps: [tr-pfsb]
links: []
created: 2026-03-09T23:37:08Z
type: task
priority: 3
assignee: Paul Sadauskas
parent: tr-09dq
tags: [phase-6, polish]
---
# Extend pager support to other commands and add --pager flag

Extract the pager helper introduced in tr-pfsb into a shared `src/pager.rs` module, extend it to other commands that produce significant output, and add `--pager`/`--no-pager` global CLI flags.

## Pager crate evaluation

Three options were considered during tr-pfsb planning:

- **Manual shell-out (chosen)**: Spawn `sh -c "$PAGER"` via `Command::new`, pipe output to stdin, handle BrokenPipe. ~20 lines of code, zero new dependencies, matches how git does it. Full control over TTY detection and env var priority (`TICKET_PAGER` > `PAGER`).

- **`pager` crate (v0.16.1)**: Fork-based — parent becomes the pager, child continues. Dead simple (`Pager::new().setup()`) but uses `fork()` which redirects ALL stdout globally, last commit Sep 2022, Linux-only, non-standard `NOPAGER` convention.

- **`minus` crate (v5.6.1)**: A full built-in terminal pager (it IS the pager, not a delegator). Rich feature set but heavy deps (crossterm ecosystem), does not respect `$PAGER` at all. Overkill for delegating to the user's preferred pager.

Manual shell-out is the right approach: simple, no new dependencies, matches user expectations around `$PAGER` configuration.

## Implementation

Extracted the pager helper from `src/commands/show.rs` into `src/pager.rs`. Commands call `pager::page_or_print(&text)` instead of `print!()`. The global `PAGER_DISABLED` atomic flag (set via `pager::set_pager_disabled(true)`) follows the same pattern as `console::set_colors_enabled`.

Commands updated: `show`, `ls`/`list`, `ready`, `blocked`, `closed`, `tree`, `dep tree`.

`query` was excluded — it outputs JSONL intended for piping to `jq` and other tools; paging would break that pipeline. `dep cycle` was also excluded — it is a short diagnostic command that exits non-zero when cycles are found.

## CLI flags

Only `--no-pager` was added (not `--pager`). Git only has `--no-pager` and there is no practical use case for forcing paging on a non-TTY pipe. `--no-pager` disables paging regardless of TTY state or env vars.
