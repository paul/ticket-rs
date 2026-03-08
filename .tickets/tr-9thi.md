---
id: tr-9thi
status: open
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

