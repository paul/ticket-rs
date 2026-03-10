---
id: tr-9j1n
status: closed
deps: []
links: []
created: 2026-03-10T00:06:50Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-09dq
tags: [polish, ux, agent-ergonomics]
---
# Add command aliases: note, new, done

Agents repeatedly fail with wrong command names because the correct ones are non-obvious. Three high-frequency patterns emerged from session history analysis:

- `ticket note <id>` (18 failures) → should route to `add-note`
- `ticket new <title>` (2 failures) → should route to `create`
- `ticket status <id> done` (1 failure) → should accept `done` as alias for `closed`

## Design

**`note` and `new` aliases (`src/cli.rs`):**

Add clap `#[command(alias = "...")]` attributes to the relevant variants in the `Commands` enum:

```rust
/// Create a new ticket.
#[command(alias = "new")]
Create { ... }

/// Append a timestamped note to a ticket.
#[command(alias = "note")]
AddNote { ... }
```

**`done` alias for `closed` (`src/ticket.rs`):**

Add `"done"` as an accepted input in `Status::from_str`, normalizing it to `Closed`:

```rust
impl std::str::FromStr for Status {
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "open" => Ok(Status::Open),
            "in_progress" => Ok(Status::InProgress),
            "closed" | "done" => Ok(Status::Closed),
            other => Err(Error::InvalidStatus { value: other.to_string() }),
        }
    }
}
```

The `Close` subcommand shorthand already hardcodes `Status::Closed` so no change needed there. The `Status` subcommand goes through `from_str`, which is the only place `"done"` needs to be handled.

## Acceptance Criteria

- `ticket note <id> 'text'` appends a note identically to `ticket add-note <id> 'text'`
- `ticket new 'title' [opts]` creates a ticket identically to `ticket create 'title' [opts]`
- `ticket status <id> done` sets status to closed without error
- `ticket --help` output lists `note` and `new` as aliases in the command list
