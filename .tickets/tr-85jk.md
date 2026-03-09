---
id: tr-85jk
status: closed
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

## Testing

Write unit tests in a `#[cfg(test)]` module in `src/commands/query.rs`. Test the serialization logic directly using in-memory `Ticket` values.

- **JSONL format**: serialize two tickets; assert the output is two lines, each a valid JSON object.
- **Empty result**: serialize an empty ticket list; assert the output is empty.
- **All fields present**: serialize a ticket with every field populated; assert the JSON object contains `id`, `status`, `deps`, `links`, `created`, `type`, `priority`, `assignee`, `external-ref`, `parent`, `tags`.
- **`deps` is a JSON array**: assert `"deps": ["dep-001"]` (not a YAML-style string).
- **`links` is a JSON array**: same as above.
- **`tags` is a JSON array**: assert `"tags": ["ui", "backend"]`.
- **`external-ref` field name**: assert the JSON key is `"external-ref"` (hyphenated), not `"external_ref"`.
- **Optional fields absent when `None`**: serialize a ticket with no `assignee`, `external-ref`, `parent`, or `tags`; assert those keys are either absent or `null` (match the bash version's behavior).
- **`created` as ISO 8601 string**: assert the `created` value is a quoted UTC datetime string.
- **Round-trip**: parse a JSON line produced by the serializer back into a `serde_json::Value`; assert no parse error.

## BDD Integration Tests

```bash
TICKET_SCRIPT=./target/debug/ticket behave features/ticket_query.feature
```

Scenarios cover: JSONL output format (one object per line), all expected field names present (including `external-ref` hyphenated), `deps`/`links`/`tags` as JSON arrays, optional fields absent when unset, and jq filter piping (requires `jq` installed). The `external-ref` key name is a common gotcha — the JSON key must be hyphenated to match the bash version's output.
