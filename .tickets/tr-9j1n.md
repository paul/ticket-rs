---
id: tr-9j1n
status: open
deps: []
links: []
created: 2026-03-10T00:06:50Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-09dq
tags: [polish, ux, agent-ergonomics]
---
# Add command aliases: note, new, done

Agents repeatedly fail with wrong command names because the correct ones are non-obvious. Three high-frequency patterns emerged from session history analysis:

- `tk note <id>` (18 failures) → should route to `add-note`
- `tk new <title>` (2 failures) → should route to `create`
- `tk status <id> done` (1 failure) → should accept `done` as alias for `closed`

All changes are in `/home/rando/.local/bin/tk`.

## Design

**Dispatcher aliases (command case statement ~line 1384):**
Add `note)` and `new)` alongside the existing entries:
```bash
new|create) shift; cmd_create "$@" ;;
note|add-note) shift; cmd_add_note "$@" ;;
```

**`done` status alias (`validate_status` / `cmd_status` ~line 237):**
Normalize the status value before validation so `done` maps to `closed`:
```bash
# In cmd_status, before calling validate_status:
[[ "$status" == "done" ]] && status="closed"
```

**Help text:**
Update `cmd_help` to surface the aliases so agents can discover them.

## Acceptance Criteria

- `tk note <id> 'text'` appends a note identically to `tk add-note <id> 'text'`
- `tk new 'title' [opts]` creates a ticket identically to `tk create 'title' [opts]`
- `tk status <id> done` sets status to closed without error
- `tk help` output mentions at least the `note` and `new` aliases

