---
id: tr-4cb4
status: closed
deps: []
links: []
created: 2026-03-10T03:23:16Z
type: feature
priority: 2
assignee: Paul Sadauskas
---
# Enhance tree command with dep sorting, dep display, tags, and terminal width truncation

Four enhancements to the tree command:

1. Sort siblings by dependency order within status groups (topological sort)
2. Display dependency IDs after title, colored by status, filtered to visible set
3. Display tags when terminal width permits
4. Detect terminal width and truncate/omit optional fields to prevent wrapping; skip truncation for piped output
