---
id: tr-ca22
status: closed
deps: []
links: []
created: 2026-03-19T16:09:08Z
type: task
priority: 2
assignee: Paul Sadauskas
---
# Validate and expand TICKET_DIR path

When TICKET_DIR points to a non-existent path, the tool silently returns no output. This needs to be fixed with proper validation and helpful errors.

## Requirements

- Expand ~ and $VAR/${VAR} in TICKET_DIR values (env var and .tickets.toml)
- On reads: if the configured dir doesn't exist, print an error (TICKET_DIR — no such path "/actual/path")
- On writes: only create the final segment if the parent already exists; fail with an error if parent doesn't exist
- When the dir exists but is empty, print '-- Ticket Dir ({dir}) is empty --' instead of silent empty output
- Print a note to stderr when auto-creating the directory on write

## Implementation Plan

1. Add expand_path() to config.rs — expands ~ and $VAR/${VAR} patterns
2. Add TicketDirNotFound and TicketDirParentNotFound error variants to error.rs
3. Validate override path in TicketStore::find_impl() — return TicketDirNotFound if dir missing
4. Fix TicketStore::ensure() to use create_dir (single level) not create_dir_all, check parent exists
5. Fix ensure_dir() to use create_dir not create_dir_all
6. Add empty-dir message to list/ready/blocked/closed commands
