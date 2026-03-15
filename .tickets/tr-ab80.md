---
id: tr-ab80
status: closed
deps: []
links: []
created: 2026-03-15T21:38:50Z
type: chore
priority: 2
assignee: Paul Sadauskas
---
# Document @-input convention in help output for create, update, and add-note

Agents struggle with long text containing shell metacharacters (backticks, double-dashes, quotes) because they don't know about the @-input convention. Add after_long_help hints to the Create, Update, and AddNote subcommands in src/cli.rs documenting stdin piping, file reading, and @@ escaping.

## Acceptance Criteria

Running tk create --help, tk update --help, and tk add-note --help each show examples of using stdin piping (-d -, @path, @@) for text fields.
