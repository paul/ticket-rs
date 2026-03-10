# ticket-rs

A git-backed issue tracker for AI agents, rewritten in Rust.

`ticket-rs` is a from-scratch Rust port of [ticket](https://github.com/wedow/ticket), the bash/awk issue tracker inspired by Steve Yegge's [beads](https://github.com/steveyegge/beads). It produces the same `.tickets/` directory of markdown files with YAML frontmatter, supports the same plugin protocol, and installs as the same `tk` command -- but ships as a single compiled binary with no runtime dependencies.

## Why a Rust port?

The original `ticket` is a portable bash script. It works everywhere, but:

- Startup latency adds up when agents call `tk` dozens of times per session
- Bash+awk makes structured data manipulation fragile
- Error messages are limited (no fuzzy suggestions, no rich context)

`ticket-rs` addresses these while staying file-format compatible. Tickets created by either tool are interchangeable.

### Additions over the original

- **Syntax highlighting** via `bat` when displaying tickets
- **Fuzzy suggestions** for mistyped IDs, statuses, and types (Jaro-Winkler similarity)
- **`update` command** for modifying any ticket field without opening an editor
- **`tree` command** for visualizing parent/child hierarchies
- **Agent-friendly aliases** for subcommands and flags that LLMs commonly confuse (e.g. `create --body` works as an alias for `--description`, `close` is aliased as `done`)

## Install

**From source (requires Rust toolchain and [just](https://github.com/casey/just)):**

```
git clone <repo-url>
cd ticket-rs
just install
```

This builds a release binary, installs it to `~/.cargo/bin/ticket`, creates a `tk` symlink, and installs zsh completions.

**Manual:**

```
cargo build --release
cp target/release/ticket ~/.local/bin/
ln -s ~/.local/bin/ticket ~/.local/bin/tk
```

**Uninstall:**

```
just uninstall
```

## Requirements

- **Build time:** Rust 2024 edition toolchain
- **Runtime:** None (single static binary)
- **Optional:** `jq` for the `query` command's filter expressions, `bat` for syntax highlighting, `$EDITOR` for the `edit` command

## Agent Setup

Add this line to your `CLAUDE.md` or `AGENTS.md`:

```
This project uses a CLI ticket system for task management. Run `tk help` when you need to use it.
```

## Usage

```
ticket - a local-first issue tracker backed by plain-text files

Usage: ticket <command> [args]

Commands:
  create [TITLE] [options]     Create a ticket, prints ID
  show <ID>                    Display full ticket content
  start <ID>                   Set status to in_progress
  close <ID>                   Set status to closed
  reopen <ID>                  Set status to open
  status <ID> <STATUS>         Set status explicitly
  dep add <ID> <DEP_ID>        Add dependency
  dep remove <ID> <DEP_ID>     Remove dependency
  dep tree [--full] <ID>       Show dependency tree
  dep cycle                    Detect dependency cycles
  link <ID> <ID> [ID...]       Link tickets together (symmetric)
  unlink <ID> <TARGET_ID>      Remove a link
  ls [--status=X] [-a X] [-T X]   List tickets with optional filters
  ready [-a X] [-T X]          Open tickets with all deps resolved
  blocked [-a X] [-T X]        Open tickets with unresolved deps
  closed [--limit=N] [-a X]    Recently closed tickets (default 20)
  update <ID> [options]        Modify ticket fields
  add-note <ID> [TEXT]         Append a timestamped note
  edit <ID>                    Open ticket in $EDITOR
  tree [ID] [-L N] [--all]     Display parent/child hierarchy
  query [FILTER]               Serialize tickets to JSON (optional jq filter)
  super <CMD> [ARGS...]        Bypass plugins, run built-in directly
  help                         Show help (includes discovered plugins)

Global flags:
  --color <auto|always|never>  When to use colored output (default: auto)
  --no-pager                   Print directly to stdout
```

All commands support partial ID matching. `tk show 5c4` will match `tr-5c46` if unambiguous.

### Command Aliases

Several commands have aliases that accept common mistakes agents make:

| Command    | Alias  |
|------------|--------|
| `create`   | `new`  |
| `close`    | `done` |
| `add-note` | `note` |
| `ls`       | `list` |

### create

```
tk create "Fix the login bug" -t bug -p 1 --tags auth,urgent
tk create --title "New feature" -d "Full description here"
```

| Flag | Alias | Description | Default |
|------|-------|-------------|---------|
| `[TITLE]` | `--title` | Ticket title (positional or flag) | |
| `-d, --description` | `--body` | Description text | |
| `--design` | | Design notes section | |
| `--acceptance` | | Acceptance criteria section | |
| `-t, --type` | | bug, feature, task, epic, chore | task |
| `-p, --priority` | | 0 (highest) through 4 (lowest) | 2 |
| `-a, --assignee` | | Assignee name | git user.name |
| `--external-ref` | | External reference (gh-123, JIRA-456) | |
| `--parent` | | Parent ticket ID | |
| `--tags` | | Comma-separated tags | |

### update

Modify any field on an existing ticket without opening an editor.

```
tk update tr-5c46 --priority 0 --add-tags critical
tk update tr-5c46 -d @design-notes.md
```

| Flag | Description |
|------|-------------|
| `--title` | Replace the title heading |
| `-d, --description` | Replace description text |
| `--design` | Replace or insert design section |
| `--acceptance` | Replace or insert acceptance criteria |
| `-p, --priority` | New priority (0-4) |
| `-t, --type` | New type |
| `-a, --assignee` | New assignee |
| `--external-ref` | New external reference |
| `--parent` | New parent ticket ID |
| `--tags` | Replace all tags |
| `--add-tags` | Merge tags (deduplicated) |
| `--remove-tags` | Remove tags (deletes field if empty) |

### Listing commands

All listing commands support `-a` (assignee) and `-T` / `--tags` (tag) filters:

```
tk ls --status open -a alice
tk ready -T backend
tk blocked
tk closed --limit 5
```

`--tags` is also aliased as `--tag` for convenience.

### Dependencies

```
tk dep add tr-1234 tr-5678    # tr-1234 depends on tr-5678
tk dep remove tr-1234 tr-5678
tk dep tree tr-1234            # show full dependency graph
tk dep tree --full tr-1234     # disable deduplication
tk dep cycle                   # find cycles among open tickets
```

The `dep tree` command also accepts `-L, --max-depth` to limit display depth.

### Links

Links are symmetric (bidirectional) and distinct from dependencies:

```
tk link tr-1234 tr-5678 tr-9abc   # link all three together
tk unlink tr-1234 tr-5678         # remove one link
```

### Tree

Display the parent/child hierarchy:

```
tk tree                # show all root tickets
tk tree tr-1234        # show subtree rooted at tr-1234
tk tree -L 2           # limit depth
tk tree --all          # include closed tickets
```

### Notes

```
tk add-note tr-1234 "Discussed approach with team"
echo "multiline note" | tk note tr-1234 -
tk note tr-1234 @meeting-notes.txt
```

### Query

Serialize all tickets to JSON, with an optional `jq` filter:

```
tk query                          # all tickets as JSON
tk query '.status == "open"'      # jq filter expression
```

Requires `jq` on PATH.

## Input Conventions

All text-heavy flags (`--description`, `--design`, `--acceptance`, and note text) support special input modes:

| Syntax | Behavior |
|--------|----------|
| `plain text` | Used as-is |
| `@path/to/file` | Read contents from file |
| `@-` or `-` | Read from stdin |
| `@@literal` | Escaped: becomes literal `@literal` |

Only one stdin source (`@-` or `-`) is allowed per invocation.

## Ticket Format

Tickets are markdown files with YAML frontmatter stored in `.tickets/`:

```
---
id: tr-5c46
status: open
type: bug
priority: 1
assignee: Alice
tags: [auth, urgent]
deps: [tr-1234]
links: [tr-9abc]
parent: tr-0001
external-ref: gh-42
created: 2026-03-10T12:00:00Z
---
# Fix the login bug

Users are unable to log in when their password contains special characters.

## Design

Escape special characters before passing to the auth library.

## Acceptance Criteria

- Users can log in with passwords containing `!@#$%^&*`
- Existing sessions are not affected

## Notes

**2026-03-10T14:30:00Z**

Discussed approach with team, agreed on the escaping strategy.
```

### ID Generation

IDs are formed from a prefix derived from the project directory name plus 4 random hex digits:

| Directory | Prefix | Example ID |
|-----------|--------|------------|
| `my-project` | `mp` | `mp-a3f1` |
| `ticket-cli-rs` | `tcr` | `tcr-0b2e` |
| `platform` | `pla` | `pla-7d4c` |

The prefix is built by splitting the directory name on `-` and `_`, taking the first letter of each segment (max 4 characters).

### Dynamic Sections in `show`

When displaying a ticket with `tk show`, additional sections are appended dynamically based on relationships:

- **Blockers** -- dependencies that are not yet closed
- **Blocking** -- other tickets that depend on this one
- **Children** -- tickets with this ticket as their parent
- **Linked** -- symmetrically linked tickets

These sections do not exist in the file on disk.

## Plugins

Executables named `ticket-<cmd>` or `tk-<cmd>` on your PATH are discovered automatically and invoked as subcommands:

```bash
# Create a plugin
cat > ~/.local/bin/tk-hello <<'EOF'
#!/bin/bash
# tk-plugin: Say hello
echo "Hello from plugin!"
EOF
chmod +x ~/.local/bin/tk-hello

tk hello   # runs tk-hello
tk help    # lists it under "Plugins"
```

**Descriptions** (shown in `tk help`):
- Scripts: add `# tk-plugin: description` in the first 10 lines
- Binaries: implement `--tk-describe` to output `tk-plugin: description`

**Environment variables passed to plugins:**
- `TICKETS_DIR` -- absolute path to the `.tickets/` directory
- `TK_SCRIPT` -- absolute path to the `ticket` binary

**Calling built-ins from plugins:**

```bash
#!/bin/bash
# tk-plugin: Custom create with extras
id=$("$TK_SCRIPT" super create "$@")
echo "Created $id, doing extra stuff..."
```

Use `tk super <cmd>` to bypass plugin discovery and run the built-in directly.

## Environment Variables

| Variable | Description |
|----------|-------------|
| `TICKETS_DIR` | Override the `.tickets/` directory location (default: walks up from cwd) |
| `TICKET_PAGER` | Pager command for `show` and other long output (takes precedence over `PAGER`) |
| `PAGER` | Fallback pager command |
| `NO_COLOR` | Disable colored output (respected automatically) |
| `CLICOLOR` | Control colored output (respected automatically) |
| `EDITOR` | Editor for `tk edit` |

## Shell Completions

Zsh completions are installed automatically by `just install`. To install manually:

```
cp completions/_tk ~/.local/share/zsh/site-functions/_tk
```

The completion script provides context-aware completion for commands, ticket IDs, statuses, and types.

## Testing

```
just test
```

This runs both the Rust unit test suite and the BDD test suite (via [behave](https://behave.readthedocs.io/), requires Python). To run just the Rust tests:

```
cargo test
```

## Development

The project uses [just](https://github.com/casey/just) as a command runner:

| Recipe | Description |
|--------|-------------|
| `just build` | Debug build |
| `just release` | Release build |
| `just check` | Check compilation without building |
| `just clippy` | Run clippy lints |
| `just fmt` | Format source code |
| `just lint` | Check formatting + clippy |
| `just test` | Rust tests + BDD tests |
| `just install` | Build, install, symlink, completions |
| `just uninstall` | Remove binary, symlink, completions |
| `just clean` | Remove build artifacts |

## License

MIT
