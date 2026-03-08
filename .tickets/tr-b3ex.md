---
id: tr-b3ex
status: open
deps: [tr-kspr]
links: []
created: 2026-03-08T06:32:21Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-13ck
tags: [phase-5, command]
---
# Implement tree command

Create src/commands/tree.rs. Implement tree [ticket-id] [options]. Display parent/child hierarchy (walks the parent field, NOT deps). Options: --max-depth/-L N (limit depth, 0 = root only), --all (include closed tickets, default is open/in_progress only), --no-color. Color by status: in_progress=cyan, open=blue, closed=dim. Sort children by status priority (in_progress < open < closed) then created_at. Detect and annotate cycles. If ticket-id provided, show subtree rooted at that ticket. If omitted, show all root tickets (those with no parent or whose parent is not in the visible set). Format: box-drawing chars (├──, └──, │). Respect NO_COLOR env var and TTY detection.

