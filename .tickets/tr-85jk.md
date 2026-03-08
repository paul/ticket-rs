---
id: tr-85jk
status: open
deps: [tr-kspr]
links: []
created: 2026-03-08T06:32:12Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-13ck
tags: [phase-5, command]
---
# Implement query command

Create src/commands/query.rs. Implement query [jq-filter]. Load all tickets, serialize each to JSON via serde_json (one JSON object per line, matching the bash version's field names and structure). If a jq filter argument is provided, pipe the JSON output through jq by shelling out to the jq binary. If jq is not installed and a filter is provided, error with a helpful message. Without a filter, just output raw JSON lines. Fields: id, status, deps, links, created, type, priority, assignee, external_ref (as external-ref in JSON), parent, tags.

