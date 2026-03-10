---
id: tr-5fgx
status: open
deps: []
links: []
created: 2026-03-10T00:07:05Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-09dq
tags: [polish, ux, agent-ergonomics]
---
# Accept --title and --body flags in tk create

Agents consistently try `tk create --title 'My Title' -d 'desc'` (7 failures in session history), but the title is a positional argument — `--title` is caught by the `-*) Unknown option` catch-all before the positional `*) title=$1` case runs. Agents also use `--body` instead of `-d`/`--description` (3 failures).

Both flags are standard CLI conventions that agents expect from tools like `gh issue create`.

Changes are in `cmd_create()` in `/home/rando/.local/bin/tk`.

## Design

Add explicit cases in `cmd_create()`'s arg parser (around line 168) before the `-*)` catch-all:
```bash
--title)      title="$2"; shift 2 ;;
--body)       description="$2"; shift 2 ;;
```

This allows both the existing positional form and the named flag form:
```bash
# Both of these should work identically:
tk create 'My Title' -d 'description'
tk create --title 'My Title' --body 'description'
```

Update `cmd_help` to reflect that `create` accepts an optional `--title` flag.

## Acceptance Criteria

- `tk create --title 'My Title'` creates a ticket with the given title
- `tk create --title 'T' --body 'desc'` sets both title and description
- `tk create --title 'T' -d 'desc'` also works (--body and -d are interchangeable)
- The existing positional form `tk create 'title' -d 'desc'` still works unchanged
- `tk create --title 'T' --unknown-flag` still errors with 'Unknown option'

