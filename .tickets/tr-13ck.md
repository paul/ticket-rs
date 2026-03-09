---
id: tr-13ck
status: in_progress
deps: [tr-3kr6]
links: []
created: 2026-03-08T06:29:07Z
type: epic
priority: 1
assignee: Paul Sadauskas
parent: tr-ketw
tags: [phase-5, query, tree]
---
# Phase 5: Query and tree commands

Implement query command (output tickets as JSON, one object per line, optional jq filter by shelling out to jq) and tree command (parent/child hierarchy display with --max-depth/-L, --all to include closed, --no-color, color by status, sorted by status priority then created_at, cycle detection).

