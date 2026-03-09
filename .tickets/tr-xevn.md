---
id: tr-xevn
status: closed
deps: [tr-3kr6]
links: []
created: 2026-03-08T06:28:52Z
type: epic
priority: 1
assignee: Paul Sadauskas
parent: tr-ketw
tags: [phase-2, graph]
---
# Phase 2: Dependency and link management

Implement dependency and link management commands: dep (add dependency), undep (remove dependency), dep tree (show dependency tree with ASCII art, dedup by default, --full to disable), dep cycle (DFS cycle detection on open/in_progress tickets), link (symmetric link between 2+ tickets), unlink (remove symmetric link). All commands support partial ID resolution.

