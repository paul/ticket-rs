---
id: tr-5fgx
status: closed
deps: []
links: []
created: 2026-03-10T00:07:05Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-09dq
tags: [polish, ux, agent-ergonomics]
---
# Accept --title and --body flags in ticket create

Agents consistently try `ticket create --title 'My Title' -d 'desc'` (7 failures in session history). The title is currently only accepted as a positional argument — `--title` is not a defined flag and clap rejects it with an error. Agents also use `--body` instead of `-d`/`--description` (3 failures).

Both flags are standard CLI conventions that agents expect from tools like `gh issue create`.

## Design

**`src/cli.rs` — `Commands::Create` variant:**

The `title` field is already `Option<String>` as a positional arg. Add `--title` as a named flag alias and `--body` as an alias for `--description`:

```rust
Commands::Create {
    /// Title for the new ticket (defaults to "Untitled").
    /// Can also be supplied as a positional argument.
    #[arg(long)]
    title: Option<String>,

    // positional form still accepted when --title is not used
    // ...
```

The cleanest approach in clap is to keep `title` as a positional `Option<String>` and add a separate `--title` flag, then merge them in `main.rs` before calling `commands::create`. Alternatively, clap's `value_name` and `index` attributes can express "positional or named":

```rust
/// Title (positional or --title).
#[arg(index = 1, long = "title")]
title: Option<String>,
```

Using `index = 1` together with `long = "title"` on the same field lets clap accept both `ticket create "My title"` and `ticket create --title "My title"`.

**`--body` alias for `--description`:**

Add a `visible_alias` on the `description` field:

```rust
/// Description text.
#[arg(short, long, visible_alias = "body")]
description: Option<String>,
```

**`main.rs`:** No changes needed — `dispatch` already passes `title.as_deref().unwrap_or("Untitled")`.

## Acceptance Criteria

- `ticket create --title 'My Title'` creates a ticket with the given title
- `ticket create --title 'T' --body 'desc'` sets both title and description
- `ticket create --title 'T' -d 'desc'` also works (`--body` and `-d` are interchangeable)
- The existing positional form `ticket create 'title' -d 'desc'` still works unchanged
- `ticket create --help` shows `--title` and `--body` in the flag list

## Notes

**2026-03-10T00:29:22Z**

Implemented via two changes. In src/cli.rs: added a separate title_flag field (long = "title", value_name = "TITLE", conflicts_with = "title") because clap forbids combining a positional index with a long name on the same field; added visible_alias = "body" to the description field. In src/main.rs: destructure title_flag in the Create arm and resolve with title_flag.as_deref().or(title.as_deref()).unwrap_or("Untitled"). All 258 tests pass.
