// Implementation of the `query` subcommand.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::{Error, Result};
use crate::store::TicketStore;
use crate::ticket::Ticket;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Serialize all tickets to JSONL, optionally piping through jq.
pub fn query(filter: Option<&str>) -> Result<()> {
    let output = query_impl(None, filter)?;
    print!("{output}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal implementation (testable via explicit start_dir)
// ---------------------------------------------------------------------------

/// Core logic for `query`.
///
/// `start_dir` is the directory from which to locate `.tickets/`. Passing
/// `None` uses the current working directory. Tests pass `Some(tempdir.path())`
/// to avoid touching the real filesystem.
///
/// Without a filter, returns JSONL (one JSON object per line). With a filter,
/// pipes the JSONL through `jq -c <filter>` and returns the filtered output.
fn query_impl(start_dir: Option<&Path>, filter: Option<&str>) -> Result<String> {
    let store = TicketStore::find(start_dir)?;
    let mut tickets = store.list_tickets();

    // Sort by ID for deterministic output.
    tickets.sort_by(|a, b| a.id.cmp(&b.id));

    let jsonl = serialize_tickets(&tickets)?;

    match filter {
        None => Ok(jsonl),
        Some(f) => pipe_through_jq(&jsonl, f),
    }
}

// ---------------------------------------------------------------------------
// Serialization helpers
// ---------------------------------------------------------------------------

/// Serialize a slice of tickets to JSONL (one JSON object per line).
///
/// Extracted for testability — unit tests call this directly with in-memory
/// `Ticket` values without touching the filesystem.
pub(crate) fn serialize_tickets(tickets: &[Ticket]) -> Result<String> {
    if tickets.is_empty() {
        return Ok(String::new());
    }

    let lines: Result<Vec<String>> = tickets
        .iter()
        .map(|t| serde_json::to_string(t).map_err(|e| Error::Io(std::io::Error::other(e))))
        .collect();

    let mut out = lines?.join("\n");
    out.push('\n');
    Ok(out)
}

/// Pipe `jsonl` through `jq -c "select(<filter>)"`, returning filtered objects.
///
/// The filter is wrapped in `select()` so that boolean expressions like
/// `.status == "open"` yield the matching objects rather than `true`/`false`.
/// If `jq` is not installed, returns a helpful error message.
fn pipe_through_jq(jsonl: &str, filter: &str) -> Result<String> {
    pipe_through_jq_binary(jsonl, filter, "jq")
}

/// Inner implementation parameterised on the jq binary path.
///
/// Extracted so that unit tests can substitute a nonexistent path to exercise
/// the "jq not installed" error branch without modifying the real filesystem.
fn pipe_through_jq_binary(jsonl: &str, filter: &str, binary: &str) -> Result<String> {
    let jq_expr = format!("select({filter})");
    let mut child = Command::new(binary)
        .args(["-c", &jq_expr])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::Io(std::io::Error::other(
                    "jq is not installed; install it to use jq filters (https://stedolan.github.io/jq/)",
                ))
            } else {
                Error::Io(e)
            }
        })?;

    // Write JSONL to jq's stdin.
    if let Some(stdin) = child.stdin.take() {
        let mut stdin = stdin;
        stdin.write_all(jsonl.as_bytes())?;
    }

    let output = child.wait_with_output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Io(std::io::Error::other(format!(
            "jq exited with status {}: {}",
            output.status,
            stderr.trim()
        ))));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    use crate::ticket::{Status, TicketType};

    // -----------------------------------------------------------------------
    // Helper
    // -----------------------------------------------------------------------

    fn make_ticket(id: &str) -> Ticket {
        Ticket {
            id: id.to_string(),
            status: Status::Open,
            deps: vec![],
            links: vec![],
            created: Utc::now(),
            ticket_type: TicketType::Task,
            priority: 2,
            assignee: None,
            external_ref: None,
            parent: None,
            tags: None,
            title: "Test".to_string(),
            body: "# Test\n".to_string(),
        }
    }

    // -----------------------------------------------------------------------
    // JSONL format: two tickets produce two lines
    // -----------------------------------------------------------------------

    #[test]
    fn jsonl_format_two_tickets_two_lines() {
        let tickets = vec![make_ticket("tr-0001"), make_ticket("tr-0002")];
        let output = serialize_tickets(&tickets).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2, "expected 2 lines\n{output}");
        for line in &lines {
            serde_json::from_str::<serde_json::Value>(line)
                .unwrap_or_else(|e| panic!("line is not valid JSON: {e}\n{line}"));
        }
    }

    // -----------------------------------------------------------------------
    // Empty result: empty slice produces empty string
    // -----------------------------------------------------------------------

    #[test]
    fn empty_ticket_list_produces_empty_output() {
        let tickets: Vec<Ticket> = vec![];
        let output = serialize_tickets(&tickets).unwrap();
        assert!(
            output.is_empty(),
            "expected empty output for empty ticket list"
        );
    }

    // -----------------------------------------------------------------------
    // All fields present when fully populated
    // -----------------------------------------------------------------------

    #[test]
    fn all_fields_present_when_populated() {
        let ticket = Ticket {
            id: "tr-full".to_string(),
            status: Status::Open,
            deps: vec!["dep-001".to_string()],
            links: vec!["lnk-001".to_string()],
            created: "2026-03-08T06:29:51Z".parse().unwrap(),
            ticket_type: TicketType::Task,
            priority: 1,
            assignee: Some("Alice".to_string()),
            external_ref: Some("JIRA-42".to_string()),
            parent: Some("tr-parent".to_string()),
            tags: Some(vec!["ui".to_string(), "backend".to_string()]),
            title: "Full ticket".to_string(),
            body: "# Full ticket\n".to_string(),
        };

        let output = serialize_tickets(&[ticket]).unwrap();
        let val: serde_json::Value = serde_json::from_str(output.trim()).unwrap();

        for field in &[
            "id", "status", "deps", "links", "type", "priority", "assignee", "parent", "tags",
        ] {
            assert!(
                val.get(field).is_some(),
                "expected field '{field}' in JSON\n{val}"
            );
        }
        assert!(
            val.get("external-ref").is_some(),
            "expected field 'external-ref' (hyphenated) in JSON\n{val}"
        );
    }

    // -----------------------------------------------------------------------
    // deps is a JSON array
    // -----------------------------------------------------------------------

    #[test]
    fn deps_is_json_array() {
        let ticket = Ticket {
            deps: vec!["dep-001".to_string()],
            ..make_ticket("tr-0001")
        };
        let output = serialize_tickets(&[ticket]).unwrap();
        let val: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
        assert!(
            val["deps"].is_array(),
            "expected 'deps' to be a JSON array\n{val}"
        );
        assert_eq!(val["deps"][0], "dep-001");
    }

    // -----------------------------------------------------------------------
    // links is a JSON array
    // -----------------------------------------------------------------------

    #[test]
    fn links_is_json_array() {
        let ticket = Ticket {
            links: vec!["lnk-001".to_string()],
            ..make_ticket("tr-0001")
        };
        let output = serialize_tickets(&[ticket]).unwrap();
        let val: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
        assert!(
            val["links"].is_array(),
            "expected 'links' to be a JSON array\n{val}"
        );
        assert_eq!(val["links"][0], "lnk-001");
    }

    // -----------------------------------------------------------------------
    // tags is a JSON array
    // -----------------------------------------------------------------------

    #[test]
    fn tags_is_json_array() {
        let ticket = Ticket {
            tags: Some(vec!["ui".to_string(), "backend".to_string()]),
            ..make_ticket("tr-0001")
        };
        let output = serialize_tickets(&[ticket]).unwrap();
        let val: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
        assert!(
            val["tags"].is_array(),
            "expected 'tags' to be a JSON array\n{val}"
        );
        assert_eq!(val["tags"][0], "ui");
        assert_eq!(val["tags"][1], "backend");
    }

    // -----------------------------------------------------------------------
    // external-ref field name is hyphenated
    // -----------------------------------------------------------------------

    #[test]
    fn external_ref_field_name_is_hyphenated() {
        let ticket = Ticket {
            external_ref: Some("GH-99".to_string()),
            ..make_ticket("tr-0001")
        };
        let output = serialize_tickets(&[ticket]).unwrap();
        assert!(
            output.contains("\"external-ref\""),
            "expected 'external-ref' (hyphenated) in JSON output\n{output}"
        );
        assert!(
            !output.contains("\"external_ref\""),
            "expected no 'external_ref' (underscored) in JSON output\n{output}"
        );
    }

    // -----------------------------------------------------------------------
    // Optional fields absent when None
    // -----------------------------------------------------------------------

    #[test]
    fn optional_fields_absent_when_none() {
        let ticket = make_ticket("tr-minimal");
        let output = serialize_tickets(&[ticket]).unwrap();
        let val: serde_json::Value = serde_json::from_str(output.trim()).unwrap();

        for field in &["assignee", "external-ref", "parent", "tags"] {
            assert!(
                val.get(field).is_none(),
                "expected field '{field}' to be absent when None\n{val}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // created is an ISO 8601 string
    // -----------------------------------------------------------------------

    #[test]
    fn created_is_iso8601_string() {
        let ticket = Ticket {
            created: "2026-03-08T06:29:51Z".parse().unwrap(),
            ..make_ticket("tr-0001")
        };
        let output = serialize_tickets(&[ticket]).unwrap();
        let val: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
        let created = val["created"].as_str().expect("created should be a string");
        assert!(
            created.contains("2026-03-08"),
            "expected ISO 8601 date in 'created'\n{created}"
        );
    }

    // -----------------------------------------------------------------------
    // Round-trip: JSON line parses back without error
    // -----------------------------------------------------------------------

    #[test]
    fn round_trip_json_parses_without_error() {
        let ticket = Ticket {
            deps: vec!["dep-001".to_string()],
            tags: Some(vec!["phase-1".to_string()]),
            assignee: Some("Alice".to_string()),
            ..make_ticket("tr-0001")
        };
        let output = serialize_tickets(&[ticket]).unwrap();
        let line = output.trim();
        let result = serde_json::from_str::<serde_json::Value>(line);
        assert!(
            result.is_ok(),
            "expected JSON line to parse without error\n{line}"
        );
    }

    // -----------------------------------------------------------------------
    // jq not installed: error message is helpful
    // -----------------------------------------------------------------------

    #[test]
    fn jq_not_installed_returns_helpful_error() {
        let err = pipe_through_jq_binary("", ".status", "/nonexistent-binary-jq").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("jq is not installed"),
            "expected 'jq is not installed' in error message, got: {msg}"
        );
    }
}
