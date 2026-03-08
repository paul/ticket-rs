---
id: tr-kpds
status: open
deps: [tr-kspr]
links: []
created: 2026-03-08T06:31:36Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-h2xw
tags: [phase-3, command]
---
# Implement closed command

Add closed subcommand to src/commands/list.rs. Show recently closed tickets sorted by file modification time (most recent first). --limit=N flag (default 20). For efficiency, only scan the N most recently modified files rather than loading all tickets. Output format: 'ID       [closed] - TITLE'. Support -a/--assignee and -T/--tag filters.

## Testing

Write unit tests in a `#[cfg(test)]` module in `src/commands/list.rs`. Use `tempfile::tempdir()` for mtime-dependent tests.

- **Shows closed tickets**: write a closed ticket; assert it appears in closed output with `[closed]` and its title.
- **Excludes open tickets**: write an open ticket; assert it does not appear in closed output.
- **Output format**: assert each line matches `"ID  [closed] - TITLE"`.
- **`--limit` respected**: write three closed tickets; call `closed --limit=1`; assert only one line of output is returned.
- **Default limit of 20**: write 25 closed tickets; assert at most 20 are returned by default.
- **Sorted by mtime (most recent first)**: write two closed tickets with different modification times; assert the more recently modified file appears first. Use `std::fs::File::set_modified` or `filetime` crate to control mtimes in tests.
- **`-a` assignee filter**: assert only the specified assignee's closed tickets appear.
- **`-T` tag filter**: assert only tagged tickets appear.
- **Empty output**: when no closed tickets exist, assert output is empty.
