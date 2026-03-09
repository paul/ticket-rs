---
id: tr-r6um
status: closed
deps: [tr-3kr6]
links: []
created: 2026-03-08T06:29:02Z
type: epic
priority: 1
assignee: Paul Sadauskas
parent: tr-ketw
tags: [phase-4, mutation]
---
# Phase 4: Mutation commands

Implement commands that modify existing tickets: add-note (append timestamped note, from arg or stdin, creates ## Notes section if missing), edit (open ticket in $EDITOR, detect TTY), update (update any field from CLI: title, description, design, acceptance, priority, type, assignee, external-ref, parent, tags with --tags/--add-tags/--remove-tags semantics).

