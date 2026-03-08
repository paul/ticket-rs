---
id: tr-kcaq
status: open
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

