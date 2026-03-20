---
id: tr-dbf8
status: closed
deps: []
links: []
created: 2026-03-19T17:13:08Z
type: feature
priority: 2
assignee: Paul Sadauskas
---
# Add tk search command

Add a `tk search <query>` subcommand that performs case-insensitive substring matching across ticket titles and bodies. By default, closed tickets are excluded (consistent with ready/blocked). Use --all to include them. Supports the same filter flags as tk ls: --status, -a/--assignee, -T/--tag, and --all to include closed tickets. Output format is identical to tk ls.

## Design

No new dependencies needed. Implemented with str::to_lowercase().contains() in the filter predicate. Files to change: src/cli.rs (add Search variant), src/commands/search.rs (new module with search(), search_impl(), format_search(), and unit tests), src/commands/mod.rs (re-export), src/main.rs (dispatch arm), src/commands/list.rs (make ticket_line pub(crate) for reuse), features/ticket_search.feature (BDD tests). Discussed and decided against fuzzy matching or full-text search: the dataset is small enough that linear substring scan is instantaneous, and fuzzy matching is poorly suited to long prose bodies.

## Acceptance Criteria

tk search csv returns tickets whose title or body contains csv (case-insensitive). Closed tickets are excluded by default. tk search csv --all includes closed tickets. --status, --assignee, --tag filters work as in tk ls. Output format matches tk ls. Empty results produce empty output. Unit tests cover all filter combinations. BDD feature file covers key scenarios. cargo test passes. cargo clippy passes.
