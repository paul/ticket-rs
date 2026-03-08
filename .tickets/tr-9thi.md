---
id: tr-9thi
status: in_progress
deps: [tr-n2ln]
links: []
created: 2026-03-08T06:29:51Z
type: task
priority: 1
assignee: Paul Sadauskas
parent: tr-3kr6
tags: [phase-1, core]
---
# Implement Ticket struct and frontmatter parser

Create src/ticket.rs. Define Ticket struct with serde Serialize/Deserialize for YAML frontmatter fields: id (String), status (enum: open, in_progress, closed), deps (Vec<String>), links (Vec<String>), created (chrono DateTime<Utc>), type/ticket_type (enum: bug, feature, task, epic, chore), priority (u8, 0-4), assignee (Option<String>), external_ref/external-ref (Option<String>), parent (Option<String>), tags (Option<Vec<String>>). Handle the hyphenated YAML field name 'external-ref' via serde rename. Parse markdown body: title (first # heading after frontmatter), description (text between title and first ## heading), and named sections (## Design, ## Acceptance Criteria, ## Notes). Provide read_from_str(content: &str) -> Result<Ticket> and write_to_string(&self) -> String methods. Must produce byte-identical YAML frontmatter ordering to the bash version for clean diffs.

## Testing

Write unit tests in a `#[cfg(test)]` module at the bottom of `src/ticket.rs`.

- **Frontmatter parsing**: parse a fixture string with all fields present; assert each field value is correct. Test each status variant (`open`, `in_progress`, `closed`) and each type variant (`bug`, `feature`, `task`, `epic`, `chore`).
- **Optional fields**: parse a minimal frontmatter (no `assignee`, `external-ref`, `parent`, `tags`) and assert those fields are `None`/empty.
- **`external-ref` rename**: verify that a YAML key `external-ref` deserializes into the Rust `external_ref` field correctly.
- **Round-trip**: call `read_from_str` then `write_to_string` and assert the output is byte-identical to the input (for a canonical fixture). This is the key correctness invariant — the output must not reorder fields or change whitespace.
- **YAML field ordering**: assert the written YAML frontmatter has fields in the exact order: `id`, `status`, `deps`, `links`, `created`, `type`, `priority`, `assignee`, `external-ref`, `parent`, `tags` (matching the bash version).
- **Title extraction**: parse markdown with a `# My Title` heading and assert `ticket.title == "My Title"`.
- **Description extraction**: assert text between the title heading and the first `##` heading is captured as the description.
- **Named section extraction**: parse a body with `## Design`, `## Acceptance Criteria`, and `## Notes` sections; assert each section's content is correctly extracted.
- **Missing section**: assert that a ticket without a `## Notes` section returns `None` (or empty) for that section without error.
- **Invalid YAML**: assert that `read_from_str` returns an `Err` for malformed frontmatter.
- **Invalid status value**: assert that an unrecognized status string (e.g., `"unknown"`) returns an `Err`.

## Notes

**2026-03-08T07:32:22Z**

Simplify body parsing: Instead of parsing description and named sections (## Design, ## Acceptance Criteria, ## Notes) into structured fields, store only 'title' (extracted from first # heading) and 'body' (raw markdown after frontmatter, kept as plaintext). This guarantees byte-identical round-trips without reconstruction risk. The 'update' command (tr-gyw0) can add section-level helpers later if needed. The BTreeMap<String, String> sections approach is dropped. Keep 'created' as DateTime<Utc> for now — local timezone support is a nice-to-have deferred for later.
