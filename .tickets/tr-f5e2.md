---
id: tr-f5e2
status: open
deps: []
links: []
created: 2026-03-15T22:21:47Z
type: chore
priority: 3
assignee: Paul Sadauskas
---
# Reduce command wrapper boilerplate

Every command in src/commands/ follows the same two-function pattern: a public fn that calls an _impl fn and pipes result to pager::page_or_print(). The _impl fn starts with TicketStore::find(start_dir)?. This pattern is repeated 15+ times.

Additionally, the resolve_id + full_id_from_path two-liner (resolving a partial ticket ID to a full string ID) appears 15+ times across tree.rs, dep.rs, update.rs, and other commands with no shared helper.

Evaluate whether a resolve_full_id(store, partial) -> Result<String> helper on TicketStore would meaningfully reduce duplication. If so, add it to store.rs and update all call sites. For the _impl pattern, assess whether a macro or higher-order function would reduce boilerplate without obscuring the code structure - only proceed if the result is clearly simpler.
