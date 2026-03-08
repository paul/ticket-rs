---
id: tr-bs17
status: open
deps: [tr-kspr]
links: []
created: 2026-03-08T06:31:08Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-xevn
tags: [phase-2, command]
---
# Implement link and unlink commands

Add to src/commands/link.rs. Implement link <id> <id> [id...]: create symmetric links between all specified tickets. For each pair, add each ID to the other's links array. Prevent duplicates. Support 2+ tickets in one call. Implement unlink <id> <target-id>: remove symmetric link between two tickets (remove from both tickets' links arrays). Validate link exists before removing. All IDs resolved via partial matching.

