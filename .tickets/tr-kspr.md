---
id: tr-kspr
status: open
deps: [tr-9thi, tr-7vfp, tr-n2ln]
links: []
created: 2026-03-08T06:29:58Z
type: task
priority: 1
assignee: Paul Sadauskas
parent: tr-3kr6
tags: [phase-1, core]
---
# Implement ticket store (directory operations)

Create src/store.rs. Implement: find_tickets_dir() that walks parent directories looking for .tickets/ (check current dir, then parent, grandparent, etc.; respect TICKETS_DIR env var override). read_ticket(id) -> Result<Ticket> to read and parse a ticket file. write_ticket(ticket) -> Result<()> to write a ticket to disk. list_tickets() -> Vec<Ticket> to read all .md files in .tickets/. resolve_id(partial: &str) -> Result<PathBuf> for partial ID matching (exact match takes precedence, partial must be unique, error on ambiguous). ensure_dir() to create .tickets/ if needed. The store should be a struct that holds the resolved tickets directory path.

