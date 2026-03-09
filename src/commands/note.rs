// add-note command: append a timestamped note to a ticket's ## Notes section.
//
// If the ticket body has no ## Notes section, one is created at the end.
// Each note entry follows this structure:
//
//   <blank line>
//   **YYYY-MM-DDTHH:MM:SSZ**
//   <blank line>
//   <note text>
//   <blank line>
//
// For empty note text the structure collapses to the timestamp-only entry:
//
//   <blank line>
//   **YYYY-MM-DDTHH:MM:SSZ**
//   <blank line>

use std::io::{self, Read};
use std::path::Path;

use chrono::{DateTime, Utc};

use crate::error::Result;
use crate::store::TicketStore;

// ---------------------------------------------------------------------------
// Pure string-manipulation helper
// ---------------------------------------------------------------------------

/// Append a timestamped note entry to `body` and return the modified body.
///
/// If `body` does not contain a `## Notes` section, one is appended first.
/// The entry format is:
///
/// ```text
/// \n**YYYY-MM-DDTHH:MM:SSZ**\n\n<note_text>\n
/// ```
///
/// When `note_text` is empty the entry is timestamp-only (blank line, timestamp
/// line, blank line) with no placeholder text line.
pub fn append_note(body: &str, note_text: &str, now: DateTime<Utc>) -> String {
    let timestamp = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let timestamp_line = format!("**{timestamp}**");

    // Entry always starts with a blank line, the timestamp, and a trailing
    // blank line.  When there is text it is inserted between the timestamp and
    // the trailing blank line.
    let entry = if note_text.is_empty() {
        format!("\n{timestamp_line}\n\n")
    } else {
        format!("\n{timestamp_line}\n\n{note_text}\n")
    };

    // Ensure the base body ends with exactly one newline before we append.
    let base = body.trim_end_matches('\n');

    // Detect an actual ## Notes heading line (not an occurrence in body text).
    let has_notes_heading = body.lines().any(|line| line.trim_end() == "## Notes");

    if has_notes_heading {
        // Append to the existing section: keep everything, then add the entry.
        format!("{base}\n{entry}")
    } else {
        // No Notes section yet — create it, then add the entry.
        format!("{base}\n\n## Notes\n{entry}")
    }
}

// ---------------------------------------------------------------------------
// Testable implementation
// ---------------------------------------------------------------------------

fn add_note_impl(start_dir: Option<&Path>, partial_id: &str, note_text: &str) -> Result<String> {
    let store = TicketStore::find(start_dir)?;

    let path = store.resolve_id(partial_id)?;
    let full_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(partial_id)
        .to_string();

    let mut ticket = store.read_ticket(&full_id)?;
    ticket.body = append_note(&ticket.body, note_text, Utc::now());
    store.write_ticket(&ticket)?;

    Ok(format!("Note added to {full_id}"))
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn add_note(id: &str, text: Option<&str>) -> Result<()> {
    let note_text = match text {
        Some(t) => t.to_string(),
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            // Trim trailing newline that shells typically append.
            buf.trim_end_matches('\n').to_string()
        }
    };

    let msg = add_note_impl(None, id, &note_text)?;
    println!("{msg}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use chrono::TimeZone;
    use std::fs;
    use tempfile::tempdir;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn fixed_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 3, 9, 12, 0, 0).unwrap()
    }

    /// Write a minimal ticket file into the temp store.
    fn write_ticket(tickets_dir: &Path, id: &str, extra_body: &str) {
        let content = format!(
            "---\nid: {id}\nstatus: open\ndeps: []\nlinks: []\ncreated: 2026-01-01T00:00:00Z\ntype: task\npriority: 2\n---\n# Test ticket\n{extra_body}"
        );
        fs::write(tickets_dir.join(format!("{id}.md")), content).unwrap();
    }

    fn make_tickets_dir(root: &Path) -> std::path::PathBuf {
        let dir = root.join(".tickets");
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // -----------------------------------------------------------------------
    // append_note — pure function tests
    // -----------------------------------------------------------------------

    #[test]
    fn creates_notes_section_when_absent() {
        let body = "# My ticket\n\nSome description.\n";
        let result = append_note(body, "A note", fixed_now());
        assert!(result.contains("## Notes"), "expected ## Notes section");
    }

    #[test]
    fn appends_to_existing_notes_section() {
        let body = "# My ticket\n\n## Notes\n\n**2026-01-01T00:00:00Z**\n\nOld note\n";
        let result = append_note(body, "New note", fixed_now());
        assert!(
            result.contains("Old note"),
            "existing note should be preserved"
        );
        assert!(result.contains("New note"), "new note should be present");
        let old_pos = result.find("Old note").unwrap();
        let new_pos = result.find("New note").unwrap();
        assert!(old_pos < new_pos, "old note should appear before new note");
    }

    #[test]
    fn timestamp_format() {
        let body = "# T\n";
        let result = append_note(body, "note", fixed_now());
        assert!(
            result.contains("**2026-03-09T12:00:00Z**"),
            "expected bold ISO 8601 timestamp, got:\n{result}"
        );
    }

    #[test]
    fn note_text_appears_after_timestamp() {
        let body = "# T\n";
        let result = append_note(body, "my text", fixed_now());
        let ts_pos = result.find("**2026-03-09T12:00:00Z**").unwrap();
        let text_pos = result.find("my text").unwrap();
        assert!(text_pos > ts_pos, "note text should appear after timestamp");
    }

    #[test]
    fn empty_note_text_no_bare_blank_line() {
        let body = "# T\n";
        let result = append_note(body, "", fixed_now());
        assert!(result.contains("## Notes"), "expected ## Notes");
        assert!(
            result.contains("**2026-03-09T12:00:00Z**"),
            "expected timestamp"
        );
        // A blank line after the timestamp is correct (it is part of the entry
        // structure).  What must NOT appear is a non-empty text placeholder —
        // i.e. no additional content between the timestamp's trailing blank line
        // and the end of the string.
        let ts_marker = "**2026-03-09T12:00:00Z**\n\n";
        let ts_end = result.find(ts_marker).unwrap() + ts_marker.len();
        let after_entry = result[ts_end..].trim();
        assert!(
            after_entry.is_empty(),
            "empty note should not append a placeholder text line, got: {after_entry:?}"
        );
    }

    #[test]
    fn blank_line_structure() {
        let body = "# T\n";
        let result = append_note(body, "the text", fixed_now());
        // We expect: ...\n\n## Notes\n\n**ts**\n\nthe text\n
        // Check relative positions of blank lines.
        assert!(
            result.contains("\n\n**2026-03-09T12:00:00Z**\n\nthe text\n"),
            "expected blank-timestamp-blank-text-blank structure, got:\n{result}"
        );
    }

    #[test]
    fn existing_content_preserved() {
        let body = "# T\n\nDescription paragraph.\n\n## Design\n\nDesign notes.\n";
        let result = append_note(body, "note", fixed_now());
        // Everything before ## Notes must be intact.
        assert!(
            result.starts_with("# T\n\nDescription paragraph.\n\n## Design\n\nDesign notes."),
            "pre-Notes content was modified:\n{result}"
        );
    }

    #[test]
    fn multiple_notes_accumulate() {
        let body = "# T\n";
        let now1 = Utc.with_ymd_and_hms(2026, 3, 9, 10, 0, 0).unwrap();
        let now2 = Utc.with_ymd_and_hms(2026, 3, 9, 11, 0, 0).unwrap();

        let after_first = append_note(body, "first note", now1);
        let after_second = append_note(&after_first, "second note", now2);

        assert!(after_second.contains("first note"), "first note missing");
        assert!(after_second.contains("second note"), "second note missing");
        assert!(
            after_second.contains("2026-03-09T10:00:00Z"),
            "first timestamp missing"
        );
        assert!(
            after_second.contains("2026-03-09T11:00:00Z"),
            "second timestamp missing"
        );

        let pos_first = after_second.find("first note").unwrap();
        let pos_second = after_second.find("second note").unwrap();
        assert!(
            pos_first < pos_second,
            "first note should precede second note"
        );
    }

    // -----------------------------------------------------------------------
    // add_note_impl — integration with store
    // -----------------------------------------------------------------------

    #[test]
    fn output_message() {
        let tmp = tempdir().unwrap();
        let tickets_dir = make_tickets_dir(tmp.path());
        write_ticket(&tickets_dir, "note-0001", "");

        let msg = add_note_impl(Some(tmp.path()), "note-0001", "some text").unwrap();
        assert_eq!(msg, "Note added to note-0001");
    }

    #[test]
    fn partial_id_resolution() {
        let tmp = tempdir().unwrap();
        let tickets_dir = make_tickets_dir(tmp.path());
        write_ticket(&tickets_dir, "note-0001", "");

        let msg = add_note_impl(Some(tmp.path()), "0001", "text").unwrap();
        assert_eq!(msg, "Note added to note-0001");

        // Verify the correct file was updated.
        let content = fs::read_to_string(tickets_dir.join("note-0001.md")).unwrap();
        assert!(content.contains("## Notes"), "file should have ## Notes");
        assert!(content.contains("text"), "file should contain note text");
    }

    #[test]
    fn non_existent_ticket() {
        let tmp = tempdir().unwrap();
        make_tickets_dir(tmp.path());

        let err = add_note_impl(Some(tmp.path()), "ghost-9999", "note").unwrap_err();
        assert!(
            matches!(err, Error::TicketNotFound { .. }),
            "expected TicketNotFound, got {err:?}"
        );
    }
}
