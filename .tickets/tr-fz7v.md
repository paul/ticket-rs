---
id: tr-fz7v
status: open
deps: [tr-siyb, tr-kspr]
links: []
created: 2026-03-08T06:30:25Z
type: task
priority: 1
assignee: Paul Sadauskas
parent: tr-3kr6
tags: [phase-1, command]
---
# Implement show command

Create src/commands/show.rs. Resolve partial ID, read ticket file, display full markdown content. Append dynamic sections computed at display time: ## Blockers (unclosed deps with [status] and title), ## Blocking (tickets that list this one in their deps), ## Children (tickets with this as parent), ## Linked (tickets in this ticket's links array, with [status] and title). If parent field is set, show parent title as inline annotation. Output through pager if stdout is TTY (TICKET_PAGER or PAGER env var). Syntax highlighting via syntect is deferred to Phase 6 but the command should work without it.

