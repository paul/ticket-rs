---
id: tr-kcaq
status: closed
deps: [tr-kspr]
links: []
created: 2026-03-08T06:31:48Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-r6um
tags: [phase-4, command]
---
# Implement add-note command

Create src/commands/note.rs. Implement add-note <id> [text]. Accept note text as a positional argument or via stdin (if no text arg provided, read from stdin). Resolve partial ID. Create '## Notes' section at end of file if it doesn't exist. Append a timestamped entry: blank line, '**YYYY-MM-DDTHH:MM:SSZ**' (ISO8601 UTC), blank line, note text, blank line. Preserve all existing file content.

## Testing

Write unit tests in a `#[cfg(test)]` module in `src/commands/note.rs`. Use `tempfile::tempdir()` for filesystem tests; factor out a `append_note(content: &str, note: &str, now: DateTime<Utc>) -> String` pure function to test string manipulation independently.

- **Creates `## Notes` section when absent**: pass a ticket body with no Notes section; assert the returned string contains `## Notes`.
- **Appends to existing `## Notes` section**: pass a body with an existing Notes section; assert the new note is appended after the existing content.
- **Timestamp format**: assert the note entry contains a timestamp matching `**YYYY-MM-DDTHH:MM:SSZ**` (ISO 8601 UTC, bold markdown).
- **Note text appears**: assert the supplied note text appears in the output after the timestamp.
- **Empty note text**: supply an empty string; assert only the timestamp entry is added (no bare empty line for the text).
- **Blank-line structure**: assert the appended entry follows the pattern: blank line, timestamp line, blank line, text, blank line.
- **Existing content preserved**: assert all content before the Notes section is byte-identical after the operation.
- **Multiple notes accumulate**: call `append_note` twice on the same content; assert both timestamps and both note texts appear, oldest first.
- **Output message**: assert stdout is `"Note added to <id>"`.
- **Partial ID resolution**: create ticket `note-0001`, call `add-note 0001 "text"`; assert the correct file is updated.
- **Non-existent ticket**: assert a `TicketNotFound` error.

## BDD Integration Tests

```bash
TICKET_SCRIPT=./target/debug/ticket behave features/ticket_notes.feature
```

Scenarios cover: adding a note (creates `## Notes` section if absent), timestamp format in bold ISO 8601, note text appears after timestamp, multiple notes accumulate in order, and partial ID resolution. The timestamp pattern `**YYYY-MM-DDTHH:MM:SSZ**` must match exactly as the BDD suite asserts it via regex.
