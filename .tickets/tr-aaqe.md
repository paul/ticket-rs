---
id: tr-aaqe
status: open
deps: [tr-kspr]
links: []
created: 2026-03-08T06:30:51Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-xevn
tags: [phase-2, command]
---
# Implement dep and undep commands

Create src/commands/dep.rs. Implement dep <id> <dep-id>: add dep-id to the deps array of ticket id. Resolve both IDs via partial matching. Prevent duplicates. Implement undep <id> <dep-id>: remove dep-id from the deps array. Validate the dependency exists before removing. Both commands update the YAML frontmatter deps field and write back.

