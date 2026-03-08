---
id: tr-04qd
status: in_progress
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

## Testing

Write unit tests in a `#[cfg(test)]` module in `src/commands/create.rs`. Use `tempfile::tempdir()` for filesystem tests.

- **Default field values**: create a ticket with only a title; assert the written file has `status: open`, `priority: 2`, `type: task`, `deps: []`, `links: []`.
- **Title set**: assert the `# Heading` in the written file matches the supplied title.
- **Default title**: create with no title argument; assert the heading is `# Untitled`.
- **`-d` description**: assert the description text appears between the title heading and the first `##` heading.
- **`--design` section**: assert a `## Design` section is created with the supplied text.
- **`--acceptance` section**: assert a `## Acceptance Criteria` section is created with the supplied text.
- **`-t` type**: create with `--type bug`; assert `type: bug` in frontmatter.
- **`-p` priority**: create with `--priority 0`; assert `priority: 0` in frontmatter.
- **`-a` assignee**: create with `--assignee "Jane"`, assert `assignee: Jane` in frontmatter.
- **`--external-ref`**: assert `external-ref: JIRA-42` appears in frontmatter.
- **`--parent` validation**: assert an error is returned when the parent ID does not resolve to an existing ticket.
- **`--tags`**: create with `--tags ui,backend`; assert `tags: [ui, backend]` in frontmatter.
- **`created` timestamp**: assert the `created` field is present and parses as a valid UTC ISO 8601 datetime.
- **`.tickets/` directory created**: run create against a temp dir with no `.tickets/` subdir; assert the dir exists after.
- **ID collision retry**: pre-seed the store with a file whose name would collide; assert create still succeeds and produces a non-colliding ID.
- **ID printed to stdout**: assert the printed output matches the ticket ID format.

## BDD Integration Tests

Once the `create` command is wired into the binary, verify it against the full scenario suite:

```bash
TICKET_SCRIPT=./target/debug/ticket behave features/ticket_creation.feature
```

All 17 scenarios in `ticket_creation.feature` must pass. These cover default field values, optional sections (description, design, acceptance criteria), assignee, type, priority, parent validation, timestamp format, and `.tickets/` directory creation on demand. The feature file is the authoritative acceptance spec — unit tests above validate logic in isolation, the BDD suite validates end-to-end observable behavior.
