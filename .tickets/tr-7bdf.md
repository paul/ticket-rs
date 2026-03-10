---
id: tr-7bdf
status: closed
deps: []
links: []
created: 2026-03-10T02:15:54Z
type: feature
priority: 2
assignee: Paul Sadauskas
---
# Add did-you-mean suggestions for invalid ticket IDs, statuses, and types

When a user provides an invalid ticket ID, status, or ticket type, show helpful suggestions rather than a bare error message.

## Design

Add the `strsim` crate for fuzzy string similarity. Add a `suggest` module with utilities for matching against ticket IDs (returning full Ticket objects) and known enum values (status, type).

Enrich three Error variants with suggestion data:
- `TicketNotFound { id, suggestions: Vec<Ticket> }`
- `InvalidStatus { value, suggestion: Option<String> }`
- `InvalidType { value, suggestion: Option<String> }`

Update `resolve_id` in store.rs: on not-found, load all tickets, compute similarity, embed top suggestions in the error.

Update `Status::from_str` and `TicketType::from_str` to compute and embed the closest suggestion before returning the error.

Plain-text `Display` for the error stays clean (IDs only, no ANSI). Color rendering lives in main.rs, which matches on the enriched error variants and uses console::style to format suggestion lines like the `ls` output (id + status + title).

## Acceptance Criteria

- `tk show tr-xyz` (no match) prints: 'Error: ticket not found' then a 'did you mean?' block with up to 3 suggestions in ls format (id + colored status + title)
- `tk status <id> in_progres` prints the error with 'did you mean: in_progress?' appended
- `tk create --type feeture` prints the error with 'did you mean: feature?' appended
- Clap continues to handle subcommand/flag typos on its own (no change)
- All existing tests pass; new tests cover the suggest module and enriched error variants
- No suggestions are shown when no similar match is found above the similarity threshold

## Implementation Plan

1. Add `strsim = "0.11"` to Cargo.toml
2. Add `src/suggest.rs` with:
   - `suggest_tickets(input, tickets, max) -> Vec<Ticket>` using Jaro-Winkler similarity on IDs
   - `suggest_keyword(input, candidates) -> Option<String>` for status/type matching
3. Add `pub mod suggest` to lib.rs
4. Update `error.rs`: add `suggestions: Vec<Ticket>` to `TicketNotFound`, `suggestion: Option<String>` to `InvalidStatus` and `InvalidType`; update Display (plain text, IDs only); update tests for new fields
5. Update `store.rs` `resolve_id`: on not-found, call `list_tickets()` then `suggest_tickets()` and embed results
6. Update `ticket.rs` `Status::from_str` and `TicketType::from_str`: call `suggest_keyword()` and embed result
7. Update `main.rs` error rendering: match on enriched variants, print colored suggestion block using console::style
