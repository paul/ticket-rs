// Ticket struct, YAML frontmatter serde, and markdown parsing/writing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::{Error, Result};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Open,
    InProgress,
    Closed,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Open => write!(f, "open"),
            Status::InProgress => write!(f, "in_progress"),
            Status::Closed => write!(f, "closed"),
        }
    }
}

impl std::str::FromStr for Status {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> crate::error::Result<Self> {
        match s {
            "open" => Ok(Status::Open),
            "in_progress" => Ok(Status::InProgress),
            "closed" => Ok(Status::Closed),
            other => Err(crate::error::Error::InvalidStatus {
                value: other.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TicketType {
    Bug,
    Feature,
    Task,
    Epic,
    Chore,
}

impl fmt::Display for TicketType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TicketType::Bug => write!(f, "bug"),
            TicketType::Feature => write!(f, "feature"),
            TicketType::Task => write!(f, "task"),
            TicketType::Epic => write!(f, "epic"),
            TicketType::Chore => write!(f, "chore"),
        }
    }
}

impl std::str::FromStr for TicketType {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> crate::error::Result<Self> {
        match s {
            "bug" => Ok(TicketType::Bug),
            "feature" => Ok(TicketType::Feature),
            "task" => Ok(TicketType::Task),
            "epic" => Ok(TicketType::Epic),
            "chore" => Ok(TicketType::Chore),
            other => Err(crate::error::Error::InvalidType {
                value: other.to_string(),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Frontmatter (deserialization only)
// ---------------------------------------------------------------------------

/// Internal struct used only for serde_yaml deserialization of the YAML block.
/// Field ordering here does not matter — ordering is controlled in write_to_string.
#[derive(Deserialize)]
struct Frontmatter {
    id: String,
    status: Status,
    #[serde(default)]
    deps: Vec<String>,
    #[serde(default)]
    links: Vec<String>,
    created: DateTime<Utc>,
    #[serde(rename = "type")]
    ticket_type: TicketType,
    priority: u8,
    assignee: Option<String>,
    #[serde(rename = "external-ref")]
    external_ref: Option<String>,
    parent: Option<String>,
    tags: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Ticket
// ---------------------------------------------------------------------------

/// A parsed ticket.
///
/// Implements `Serialize` for the frontmatter fields only — `title` and `body`
/// are excluded. This makes `serde_json::to_string(&ticket)` produce the JSON
/// representation used by the `query` command, with field names matching the
/// bash version (e.g. `"type"`, `"external-ref"`).
///
/// Deserialization is handled via the private `Frontmatter` DTO; use
/// `Ticket::read_from_str` to parse a ticket file.
#[derive(Debug, Clone, Serialize)]
pub struct Ticket {
    pub id: String,
    pub status: Status,
    pub deps: Vec<String>,
    pub links: Vec<String>,
    pub created: DateTime<Utc>,
    #[serde(rename = "type")]
    pub ticket_type: TicketType,
    pub priority: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(rename = "external-ref", skip_serializing_if = "Option::is_none")]
    pub external_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// First `# ` heading extracted from the markdown body.
    #[serde(skip)]
    pub title: String,
    /// Raw markdown content after the closing `---` delimiter, verbatim.
    #[serde(skip)]
    pub body: String,
}

impl Ticket {
    /// Parse a ticket from its full file contents (frontmatter + markdown body).
    pub fn read_from_str(content: &str) -> Result<Ticket> {
        // Strip the opening ---
        let rest = content.strip_prefix("---\n").ok_or_else(|| {
            Error::Yaml(serde_yaml::from_str::<serde_yaml::Value>("invalid: [").unwrap_err())
        })?;

        // Split on the closing ---
        let (yaml, body) = rest.split_once("\n---\n").ok_or_else(|| {
            Error::Yaml(serde_yaml::from_str::<serde_yaml::Value>("invalid: [").unwrap_err())
        })?;

        let fm: Frontmatter = serde_yaml::from_str(yaml)?;

        if fm.priority > 4 {
            return Err(Error::InvalidPriority {
                value: fm.priority.to_string(),
            });
        }

        // Extract title from the first `# ` heading in the body.
        let title = body
            .lines()
            .find_map(|line| line.strip_prefix("# "))
            .unwrap_or("")
            .to_string();

        Ok(Ticket {
            id: fm.id,
            status: fm.status,
            deps: fm.deps,
            links: fm.links,
            created: fm.created,
            ticket_type: fm.ticket_type,
            priority: fm.priority,
            assignee: fm.assignee,
            external_ref: fm.external_ref,
            parent: fm.parent,
            tags: fm.tags,
            title,
            body: body.to_string(),
        })
    }

    /// Serialize the ticket back to its full file contents.
    ///
    /// Field order matches the bash version exactly. Optional fields are omitted
    /// when `None`. Arrays use flow style: `[a, b]` or `[]`.
    pub fn write_to_string(&self) -> String {
        let mut out = String::new();

        out.push_str("---\n");
        out.push_str(&format!("id: {}\n", self.id));
        out.push_str(&format!("status: {}\n", self.status));
        out.push_str(&format!("deps: [{}]\n", self.deps.join(", ")));
        out.push_str(&format!("links: [{}]\n", self.links.join(", ")));
        out.push_str(&format!(
            "created: {}\n",
            self.created.format("%Y-%m-%dT%H:%M:%SZ")
        ));
        out.push_str(&format!("type: {}\n", self.ticket_type));
        out.push_str(&format!("priority: {}\n", self.priority));
        if let Some(ref assignee) = self.assignee {
            out.push_str(&format!("assignee: {}\n", assignee));
        }
        if let Some(ref ext_ref) = self.external_ref {
            out.push_str(&format!("external-ref: {}\n", ext_ref));
        }
        if let Some(ref parent) = self.parent {
            out.push_str(&format!("parent: {}\n", parent));
        }
        if let Some(ref tags) = self.tags {
            out.push_str(&format!("tags: [{}]\n", tags.join(", ")));
        }
        out.push_str("---\n");
        out.push_str(&self.body);

        out
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A canonical fixture with all fields present. Used for round-trip and
    /// ordering tests. Must be byte-identical to what write_to_string produces.
    const FULL_FIXTURE: &str = "\
---
id: tr-abcd
status: in_progress
deps: [tr-1111, tr-2222]
links: [tr-3333]
created: 2026-03-08T06:29:51Z
type: task
priority: 1
assignee: Paul Sadauskas
external-ref: JIRA-42
parent: tr-3kr6
tags: [phase-1, core]
---
# My Ticket Title

This is the description paragraph.

## Testing

Some test notes here.
";

    #[test]
    fn parse_all_fields() {
        let t = Ticket::read_from_str(FULL_FIXTURE).unwrap();
        assert_eq!(t.id, "tr-abcd");
        assert_eq!(t.status, Status::InProgress);
        assert_eq!(t.deps, vec!["tr-1111", "tr-2222"]);
        assert_eq!(t.links, vec!["tr-3333"]);
        assert_eq!(
            t.created.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "2026-03-08T06:29:51Z"
        );
        assert_eq!(t.ticket_type, TicketType::Task);
        assert_eq!(t.priority, 1);
        assert_eq!(t.assignee.as_deref(), Some("Paul Sadauskas"));
        assert_eq!(t.external_ref.as_deref(), Some("JIRA-42"));
        assert_eq!(t.parent.as_deref(), Some("tr-3kr6"));
        assert_eq!(
            t.tags
                .as_ref()
                .map(|v| v.iter().map(String::as_str).collect::<Vec<_>>()),
            Some(vec!["phase-1", "core"])
        );
        assert_eq!(t.title, "My Ticket Title");
    }

    #[test]
    fn parse_status_variants() {
        for (s, expected) in [
            ("open", Status::Open),
            ("in_progress", Status::InProgress),
            ("closed", Status::Closed),
        ] {
            let content = format!(
                r#"---
id: tr-test
status: {s}
deps: []
links: []
created: 2026-01-01T00:00:00Z
type: task
priority: 2
---
# T
"#
            );
            let t = Ticket::read_from_str(&content).unwrap();
            assert_eq!(
                t.status, expected,
                "status variant '{s}' did not parse correctly"
            );
        }
    }

    #[test]
    fn parse_type_variants() {
        for (s, expected) in [
            ("bug", TicketType::Bug),
            ("feature", TicketType::Feature),
            ("task", TicketType::Task),
            ("epic", TicketType::Epic),
            ("chore", TicketType::Chore),
        ] {
            let content = format!(
                r#"---
id: tr-test
status: open
deps: []
links: []
created: 2026-01-01T00:00:00Z
type: {s}
priority: 2
---
# T
"#
            );
            let t = Ticket::read_from_str(&content).unwrap();
            assert_eq!(
                t.ticket_type, expected,
                "type variant '{s}' did not parse correctly"
            );
        }
    }

    #[test]
    fn parse_optional_fields_missing() {
        let content = r#"---
id: tr-min
status: open
deps: []
links: []
created: 2026-01-01T00:00:00Z
type: task
priority: 2
---
# Minimal
"#;
        let t = Ticket::read_from_str(content).unwrap();
        assert!(t.assignee.is_none());
        assert!(t.external_ref.is_none());
        assert!(t.parent.is_none());
        assert!(t.tags.is_none());
    }

    #[test]
    fn external_ref_rename() {
        let content = r#"---
id: tr-ext
status: open
deps: []
links: []
created: 2026-01-01T00:00:00Z
type: task
priority: 2
external-ref: GH-99
---
# Ext
"#;
        let t = Ticket::read_from_str(content).unwrap();
        assert_eq!(t.external_ref.as_deref(), Some("GH-99"));
    }

    #[test]
    fn round_trip_byte_identical() {
        let t = Ticket::read_from_str(FULL_FIXTURE).unwrap();
        let output = t.write_to_string();
        assert_eq!(
            output, FULL_FIXTURE,
            "round-trip output differed from input"
        );
    }

    #[test]
    fn yaml_field_ordering() {
        let t = Ticket::read_from_str(FULL_FIXTURE).unwrap();
        let output = t.write_to_string();
        // Extract just the frontmatter lines.
        let fm_lines: Vec<&str> = output
            .lines()
            .skip(1) // skip opening ---
            .take_while(|l| *l != "---")
            .collect();
        let keys: Vec<&str> = fm_lines
            .iter()
            .map(|l| l.split(':').next().unwrap())
            .collect();
        assert_eq!(
            keys,
            [
                "id",
                "status",
                "deps",
                "links",
                "created",
                "type",
                "priority",
                "assignee",
                "external-ref",
                "parent",
                "tags"
            ],
            "YAML field ordering did not match expected"
        );
    }

    #[test]
    fn title_extraction() {
        let content = r#"---
id: tr-t
status: open
deps: []
links: []
created: 2026-01-01T00:00:00Z
type: task
priority: 2
---
# My Title

Some text.
"#;
        let t = Ticket::read_from_str(content).unwrap();
        assert_eq!(t.title, "My Title");
    }

    #[test]
    fn missing_section_no_error() {
        // A ticket with no ## Notes section should parse without error.
        let content = r#"---
id: tr-ns
status: open
deps: []
links: []
created: 2026-01-01T00:00:00Z
type: task
priority: 2
---
# No Notes

Just a description.
"#;
        assert!(Ticket::read_from_str(content).is_ok());
    }

    #[test]
    fn invalid_priority_out_of_range_returns_err() {
        let content = r#"---
id: tr-bad
status: open
deps: []
links: []
created: 2026-01-01T00:00:00Z
type: task
priority: 5
---
# T
"#;
        assert!(
            Ticket::read_from_str(content).is_err(),
            "expected Err for priority value out of range (5)"
        );
    }

    #[test]
    fn invalid_yaml_returns_err() {
        let content = r#"---
id: [unclosed
status: open
---
# T
"#;
        assert!(
            Ticket::read_from_str(content).is_err(),
            "expected Err for malformed YAML"
        );
    }

    #[test]
    fn invalid_status_value_returns_err() {
        let content = r#"---
id: tr-bad
status: unknown
deps: []
links: []
created: 2026-01-01T00:00:00Z
type: task
priority: 2
---
# T
"#;
        assert!(
            Ticket::read_from_str(content).is_err(),
            "expected Err for unrecognized status value"
        );
    }

    /// Parse real on-disk ticket files and verify they round-trip byte-identically.
    /// This catches any formatting assumptions that diverge from the bash version.
    #[test]
    fn round_trip_real_files() {
        let files = [
            // Has no optional fields (no assignee, parent, tags, external-ref)
            ".tickets/tr-ketw.md",
            // Has assignee, parent, tags but no external-ref
            ".tickets/tr-9thi.md",
            // Has assignee, parent, tags
            ".tickets/tr-3kr6.md",
        ];
        for path in &files {
            let content =
                std::fs::read_to_string(path).unwrap_or_else(|_| panic!("could not read {path}"));
            let ticket = Ticket::read_from_str(&content)
                .unwrap_or_else(|e| panic!("failed to parse {path}: {e}"));
            let output = ticket.write_to_string();
            assert_eq!(output, content, "round-trip failed for {path}");
        }
    }
}
