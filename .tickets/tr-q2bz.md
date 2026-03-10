---
id: tr-q2bz
status: closed
deps: []
links: []
created: 2026-03-10T00:07:53Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-09dq
tags: [polish, ux, agent-ergonomics]
---
# Add --tags alias to ticket ls/list

The `ls`/`list` command currently exposes a `--tag` flag (short: `-T`) for filtering by tag. Agents consistently try `--tags` instead (7 silent failures in session history), because `ticket create` uses `--tags` and agents expect the same name. The plural form `--tags` is more natural and consistent with the rest of the CLI.

## Design

**`src/cli.rs` — `Commands::Ls`, `Commands::Ready`, `Commands::Blocked`, `Commands::Closed`:**

Add `--tags` as a visible alias on the `tag` field in each of these variants:

```rust
/// Filter by tag.
#[arg(short = 'T', long, visible_alias = "tags")]
tag: Option<String>,
```

This makes `ticket list --tags spike`, `ticket list --tag spike`, and `ticket list -T spike` all equivalent, while keeping `--tag` as the canonical name in `--help` output (or swap them so `--tags` is canonical, matching `create`).

Consider making `--tags` the primary long name (matching `create`) and `--tag` the alias instead, for consistency:

```rust
/// Filter by tag.
#[arg(short = 'T', long = "tags", visible_alias = "tag")]
tag: Option<String>,
```

Apply to all four commands that have a `tag` filter field: `Ls`, `Ready`, `Blocked`, `Closed`.

**`src/main.rs`:** No changes needed — `tag` is already passed through as `tag.as_deref()`.

## Acceptance Criteria

- `ticket list --tags spike` filters to tickets tagged 'spike'
- `ticket list --tag spike` also filters correctly
- `ticket list -T spike` still works
- `ticket ready --tags spike` also works (same alias applied to all filter commands)
- `ticket list --help` shows the flag (as either `--tags` or `--tag` with the other as alias)
