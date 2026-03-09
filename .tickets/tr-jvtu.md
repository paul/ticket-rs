---
id: tr-jvtu
status: open
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

Extract the pager helper from `src/commands/show.rs` into a shared `src/pager.rs` module. Commands call `pager::page_output(&text)` instead of `print!()`.

Commands to consider for paging (large output): `ls`/`list`, `tree`, `dep tree`, `dep cycle`, `query`.

## CLI flags

Add `--pager` / `--no-pager` global flags to `Cli` in `src/cli.rs` (analogous to `--color`). `--no-pager` disables paging regardless of TTY or env vars. `--pager` forces paging even when stdout is not a TTY (useful for testing).
