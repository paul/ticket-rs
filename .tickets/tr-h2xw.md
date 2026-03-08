---
id: tr-h2xw
status: open
deps: [tr-3kr6]
links: []
created: 2026-03-08T06:28:56Z
type: epic
priority: 1
assignee: Paul Sadauskas
parent: tr-ketw
tags: [phase-3, listing]
---
# Phase 3: Listing and filtered views

Implement listing commands with filter support: ls/list (list tickets, --status, -a/--assignee, -T/--tag filters), ready (open/in_progress tickets with all deps closed, sorted by priority then ID, shows priority badge), blocked (open/in_progress tickets with unresolved deps, shows open blockers), closed (recently closed tickets by mtime, --limit=N default 20).

