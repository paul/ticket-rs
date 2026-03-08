---
id: tr-n2ln
status: closed
deps: []
links: []
created: 2026-03-08T06:29:36Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-3kr6
tags: [phase-1, core]
---
# Implement error types

Create src/error.rs with a unified error enum covering: ticket not found, ambiguous ID match (multiple matches), tickets directory not found, YAML parse errors, IO errors, invalid status/type/priority values. Implement Display and std::error::Error. Consider using thiserror crate or manual impl.

## Testing

Write unit tests in a `#[cfg(test)]` module at the bottom of `src/error.rs`.

- **`Display` for `TicketNotFound`**: assert the formatted string contains the given ID, e.g. `"ticket 'abc-1234' not found"`.
- **`Display` for `AmbiguousId`**: assert the formatted string contains the partial ID and lists the matching candidates.
- **`Display` for `TicketsNotFound`**: assert the formatted string contains `"no .tickets directory found"` or similar.
- **`Display` for `InvalidStatus`**: assert the message names the bad value and lists the valid options (`open`, `in_progress`, `closed`).
- **`Display` for `InvalidType`** and **`InvalidPriority`**: same pattern — bad value in message, valid options listed.
- **`From<std::io::Error>`**: construct an `io::Error` and convert it; assert the result is the `Io` variant and that its `Display` output is non-empty.
- **`From<serde_yaml::Error>`**: similar — convert a YAML parse error and assert the `Yaml` variant is produced.
- **`std::error::Error` impl**: call `.source()` on a wrapped IO error and assert it returns `Some`.
