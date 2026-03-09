---
id: tr-pfsb
status: closed
deps: [tr-fz7v]
links: []
created: 2026-03-08T06:32:48Z
type: task
priority: 3
assignee: Paul Sadauskas
parent: tr-09dq
tags: [phase-6, polish]
---
# Implement pager support

Add pager support to show command output. Check TICKET_PAGER env var first, then PAGER, then fall back to no pager. Only page when stdout is a TTY — use `console::Term::stdout().is_term()` for TTY detection rather than rolling it manually. Pipe the full show output through the pager command via a child process. Handle pager exit gracefully (broken pipe is not an error). Shell out to the pager rather than using a pager crate.

## Notes

Shelling out to the pager (via `sh -c "$PAGER"`) is the right approach — no pager crate needed. Three crates were evaluated:

- **`pager` (v0.16.1)**: Fork-based, redirects all stdout globally, last commit Sep 2022, Linux-only. The fork approach is too coarse-grained for selective paging.
- **`minus` (v5.6.1)**: A full built-in terminal pager with its own UI. Does not delegate to `$PAGER` at all — overkill and wrong model for this use case.
- **Manual shell-out**: ~20 lines, zero new dependencies, matches git's behavior exactly.

Follow-up work (extract to shared module, extend to other commands, `--pager`/`--no-pager` flags) is tracked in tr-jvtu.

## BDD Integration Tests

Pager behavior is not exercised by the BDD suite — all tests run with `stdin=subprocess.DEVNULL` which makes stdout non-TTY, so the pager code path is never triggered. Validate the pager manually in a TTY session.

However, run the full `ticket_show.feature` suite to confirm pager-bypass (non-TTY) still works correctly after integration:

```bash
TICKET_SCRIPT=./target/debug/ticket behave features/ticket_show.feature
```

