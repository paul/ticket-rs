# Changelog

## [Unreleased]

### Added

- Add `tk search` for case-insensitive substring matching across ticket titles and bodies, with `--all`, `--status`, `--assignee`, and `--tags` filters matching `tk ls` behavior (tr-dbf8)
- Expand `~`, `$VAR`, and `${VAR}` in `TICKET_DIR` values from env vars and `.tickets.toml`, so paths like `~/Code/myapp/.tickets` now work as expected (tr-ca22)

### Changed

- When `TICKET_DIR` points to a path that does not exist, the tool now exits with an error (`TICKET_DIR — no such path "..."`) instead of silently producing no output (tr-ca22)
- `tk create` with a configured `TICKET_DIR` whose parent exists now creates only the final path segment rather than the full tree; if the parent is missing it exits with a clear error (tr-ca22)
- `tk ls`, `ready`, `blocked`, `closed`, and `tree` now print `-- Ticket Dir (...) is empty --` when the store exists but contains no tickets, instead of producing no output (tr-ca22)
- `tk search` now shows the same `-- Ticket Dir (...) is empty --` message as `tk ls` when the ticket directory exists but contains no tickets, so empty stores are distinct from no-match results (tr-dbf8)
- Closed dependencies are now shown in `tk tree` output with dim and strikethrough styling instead of being hidden, indicating they are resolved and not blocking (tr-889d)
- Refactor duplicated assignee/tag filter logic in `ls`, `ready`, `blocked`, and `closed` into `Ticket::has_tag` and `Ticket::matches_filters` helpers (tr-d9da)
- Refactor duplicated sort logic into `Status::sort_key` and `Ticket::sort_cmp`; `ls`, `ready`, and `blocked` now sort by status first (in_progress before open), matching `tree` ordering (tr-9e03)
- Refactor ticket-line rendering into a shared `format` module; `ls`, `ready`, `blocked`, `closed`, and `dep tree` now use the same colored `{id} {priority} {status} {title} [{deps}] {#tags}` format as `tree`, with terminal-width truncation when stdout is a TTY (tr-533b)

## [20260315] - 2026-03-15

### Added

- Add input convention hints to `--help` output for `create`, `update`, and `add-note` (tr-ab80)
- Add `.tickets.toml` configuration support for project-local prefix and directory overrides ([`ef34780`](https://github.com/paul/ticket-rs/commit/ef34780))
- Add `show-config` command to inspect resolved configuration and value sources ([`0ace6b2`](https://github.com/paul/ticket-rs/commit/0ace6b2))
- Add dynamic shell completions for bash, zsh, and fish via `clap_complete` ([`250b2ce`](https://github.com/paul/ticket-rs/commit/250b2ce))
- Add bare ticket ID fallback to `show` command (tr-8496) ([`7dbb062`](https://github.com/paul/ticket-rs/commit/7dbb062))
- Add `add` as alias for `create` command (tr-8496) ([`cdddd49`](https://github.com/paul/ticket-rs/commit/cdddd49))
- Add priority display and sorting to `tree` command (tr-4936) ([`cf86ac6`](https://github.com/paul/ticket-rs/commit/cf86ac6))
- Add `--version` flag showing build date ([`d2d6107`](https://github.com/paul/ticket-rs/commit/d2d6107))

### Changed

- Update parse error output to include full subcommand help (tr-9c78) ([`0187058`](https://github.com/paul/ticket-rs/commit/0187058))

### Fixed

- Fix `show` command rejecting unknown flags ([`acfc633`](https://github.com/paul/ticket-rs/commit/acfc633))

## [20260310] - 2026-03-10

### Added

Initial release. Add functionality from bash `ticket` tool implemented.
