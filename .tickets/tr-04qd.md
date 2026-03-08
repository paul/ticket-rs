---
id: tr-04qd
status: open
deps: [tr-siyb, tr-kspr]
links: []
created: 2026-03-08T06:30:16Z
type: task
priority: 1
assignee: Paul Sadauskas
parent: tr-3kr6
tags: [phase-1, command]
---
# Implement create command

Create src/commands/create.rs. Accept options: title (positional), -d/--description, --design, --acceptance, -t/--type (default: task), -p/--priority (default: 2), -a/--assignee (default: git user.name via git config), --external-ref, --parent (validate exists via partial ID resolution), --tags (comma-separated). Generate ID via id.rs, create .tickets/ dir if needed, write markdown file with YAML frontmatter (status: open, deps: [], links: [], created: now UTC ISO8601). Print the new ticket ID to stdout. Must check that generated ID doesn't collide with existing file — if it does, generate a new random suffix and retry.

