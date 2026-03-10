---
id: tr-q2bz
status: open
deps: []
links: []
created: 2026-03-10T00:07:53Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-09dq
tags: [polish, ux, agent-ergonomics]
---
# Fix ticket-list tag flag parsing (--tags, --tag <val>, reject unknowns)

The `ticket-list` plugin has two tag-filtering problems that cause silent failures:

1. **Wrong flag name**: The plugin uses `--tag=<val>` or `-T <val>`, but `tk help` shows `--tags` (matching the `create` command). Agents consistently try `tk list --tags spike` — which silently ignores the flag and returns all tickets unfiltered.

2. **Space-separated `--tag` not supported**: The parser handles `--tag=value` (with `=`) but not `--tag value` (space-separated). Agents naturally try the space form.

3. **Unknown flags silently swallowed**: The `*) shift ;;` catch-all drops unrecognized options without error, masking typos.

From session history: `tk list --tags spike` was tried 7 times, always returning unfiltered results with no indication of the problem.

File: `/home/rando/.local/bin/ticket-list`

## Design

Replace the current arg parser in `ticket-list`:
```bash
# Current (broken):
--tag=*) tag_filter="${1#--tag=}"; shift ;;
-T) tag_filter="$2"; shift 2 ;;
*) shift ;;   # silently swallows unknown flags

# Fixed:
-T|--tag|--tags) tag_filter="$2"; shift 2 ;;
--tag=*|--tags=*) tag_filter="${1#--tag*=}"; shift ;;
--help|-h) echo 'Usage: tk list [-T tag] [--tags tag] [--status=status] [-a assignee]'; exit 0 ;;
*) echo "Unknown option: $1" >&2; exit 1 ;;
```

This makes all three forms equivalent:
```bash
tk list -T spike
tk list --tag spike
tk list --tags spike
tk list --tags=spike
```

## Acceptance Criteria

- `tk list --tags spike` filters to tickets tagged 'spike' (was: returned all tickets)
- `tk list --tag spike` (space-separated) filters correctly (was: silently ignored)
- `tk list --tag=spike` (equals-form) still works
- `tk list -T spike` still works
- `tk list --nonexistent-flag` exits with an error message (was: silently ignored)

