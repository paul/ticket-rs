// Implementation of the `update` subcommand.
//
// Modifies any combination of frontmatter fields and body sections in a single
// operation.  All body manipulation is delegated to pure string-manipulation
// helpers (update_title, replace_description, replace_section, apply_tag_ops)
// so they can be unit-tested without touching the filesystem.
//
// Section insertion order when creating new sections:
//   ## Design < ## Acceptance Criteria < ## Notes
//
// Tag operations:
//   --tags         replace all tags (mutually exclusive with --add-tags/--remove-tags)
//   --add-tags     merge provided tags into existing set (deduplicated, order preserved)
//   --remove-tags  remove provided tags; delete the field when the result is empty

use std::path::Path;

use crate::error::{Error, Result};
use crate::store::TicketStore;
use crate::ticket::TicketType;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Update a ticket's fields and print its ID to stdout.
#[allow(clippy::too_many_arguments)]
pub fn update(
    id: &str,
    title: Option<&str>,
    description: Option<&str>,
    design: Option<&str>,
    acceptance: Option<&str>,
    priority: Option<&str>,
    ticket_type: Option<&str>,
    assignee: Option<&str>,
    external_ref: Option<&str>,
    parent: Option<&str>,
    tags: Option<&str>,
    add_tags: Option<&str>,
    remove_tags: Option<&str>,
) -> Result<()> {
    update_with_writer(
        &mut std::io::stdout(),
        None,
        id,
        title,
        description,
        design,
        acceptance,
        priority,
        ticket_type,
        assignee,
        external_ref,
        parent,
        tags,
        add_tags,
        remove_tags,
    )
}

/// Testable entry point that writes output to an arbitrary writer.
#[allow(clippy::too_many_arguments)]
fn update_with_writer(
    out: &mut dyn std::io::Write,
    start_dir: Option<&std::path::Path>,
    id: &str,
    title: Option<&str>,
    description: Option<&str>,
    design: Option<&str>,
    acceptance: Option<&str>,
    priority: Option<&str>,
    ticket_type: Option<&str>,
    assignee: Option<&str>,
    external_ref: Option<&str>,
    parent: Option<&str>,
    tags: Option<&str>,
    add_tags: Option<&str>,
    remove_tags: Option<&str>,
) -> Result<()> {
    let full_id = update_impl(
        start_dir,
        id,
        title,
        description,
        design,
        acceptance,
        priority,
        ticket_type,
        assignee,
        external_ref,
        parent,
        tags,
        add_tags,
        remove_tags,
    )?;
    writeln!(out, "{full_id}").map_err(crate::error::Error::Io)
}

// ---------------------------------------------------------------------------
// Internal implementation (testable via explicit start_dir)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn update_impl(
    start_dir: Option<&Path>,
    partial_id: &str,
    title: Option<&str>,
    description: Option<&str>,
    design: Option<&str>,
    acceptance: Option<&str>,
    priority: Option<&str>,
    ticket_type: Option<&str>,
    assignee: Option<&str>,
    external_ref: Option<&str>,
    parent: Option<&str>,
    tags: Option<&str>,
    add_tags: Option<&str>,
    remove_tags: Option<&str>,
) -> Result<String> {
    let store = TicketStore::find(start_dir)?;

    // Resolve partial ID to a full ID.
    let path = store.resolve_id(partial_id)?;
    let full_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(partial_id)
        .to_string();

    let mut ticket = store.read_ticket(&full_id)?;

    // --- Validate and resolve parent (must exist) --------------------------

    let new_parent: Option<Option<String>> = match parent {
        None => None, // not requested
        Some(partial) => {
            let parent_path = store.resolve_id(partial)?;
            let stem = parent_path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| Error::TicketNotFound {
                    id: partial.to_string(),
                })?;
            Some(Some(stem.to_string()))
        }
    };

    // --- Parse and validate new priority / type ----------------------------

    let new_priority: Option<u8> = match priority {
        None => None,
        Some(s) => {
            let n: u8 = s.parse().map_err(|_| Error::InvalidPriority {
                value: s.to_string(),
            })?;
            if n > 4 {
                return Err(Error::InvalidPriority {
                    value: s.to_string(),
                });
            }
            Some(n)
        }
    };

    let new_type: Option<TicketType> = match ticket_type {
        None => None,
        Some(s) => Some(parse_ticket_type(s)?),
    };

    // --- Apply frontmatter mutations ----------------------------------------

    if let Some(p) = new_priority {
        ticket.priority = p;
    }
    if let Some(t) = new_type {
        ticket.ticket_type = t;
    }
    if let Some(a) = assignee {
        ticket.assignee = Some(a.to_string());
    }
    if let Some(r) = external_ref {
        ticket.external_ref = Some(r.to_string());
    }
    if let Some(new_parent_val) = new_parent {
        ticket.parent = new_parent_val;
    }

    // --- Apply tag mutations ------------------------------------------------

    ticket.tags = apply_tag_ops(ticket.tags.clone(), tags, add_tags, remove_tags);

    // --- Apply body mutations -----------------------------------------------

    if let Some(new_title) = title {
        ticket.body = update_title(&ticket.body, new_title);
        ticket.title = new_title.to_string();
    }
    if let Some(new_desc) = description {
        ticket.body = replace_description(&ticket.body, new_desc);
    }
    if let Some(design_text) = design {
        ticket.body = replace_section(&ticket.body, "Design", design_text);
    }
    if let Some(acc_text) = acceptance {
        ticket.body = replace_section(&ticket.body, "Acceptance Criteria", acc_text);
    }

    store.write_ticket(&ticket)?;

    Ok(full_id)
}

// ---------------------------------------------------------------------------
// Pure string-manipulation helpers
// ---------------------------------------------------------------------------

/// Replace the first `# <title>` heading in `body` with `# <new_title>`.
///
/// If no `# ` heading is found, the body is returned unchanged.
pub fn update_title(body: &str, new_title: &str) -> String {
    let mut lines: Vec<&str> = body.lines().collect();
    let mut replaced = false;
    for line in lines.iter_mut() {
        if line.starts_with("# ") && !replaced {
            *line = new_title; // will be re-formatted below
            replaced = true;
            break;
        }
    }

    if !replaced {
        return body.to_string();
    }

    // Re-assemble, but the matched line needs the "# " prefix restored.
    // We work on the raw string instead to preserve trailing newline.
    let mut result = String::new();
    let mut heading_replaced = false;
    for line in body.lines() {
        if line.starts_with("# ") && !heading_replaced {
            result.push_str(&format!("# {new_title}"));
            heading_replaced = true;
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }

    // Preserve trailing newline behaviour: if body ends with '\n' we already
    // added one for the last line; if it doesn't, the lines() iterator drops
    // it and we should not add an extra one.
    if body.ends_with('\n') {
        result
    } else {
        // strip the extra trailing newline we unconditionally added
        result.trim_end_matches('\n').to_string()
    }
}

/// Replace the description — the text between the `# Title` line and the
/// first `## ` heading (or end of body if there are no `## ` headings).
///
/// If the body has no `# ` title line, the body is returned unchanged.
/// `new_desc` is placed after a blank line following the title, followed by
/// a trailing newline, then the rest of the body starting from the first
/// `## ` heading (if any).
pub fn replace_description(body: &str, new_desc: &str) -> String {
    // Find the title line index.
    let lines: Vec<&str> = body.lines().collect();
    let title_idx = match lines.iter().position(|l| l.starts_with("# ")) {
        Some(i) => i,
        None => return body.to_string(),
    };

    // Find the first ## heading after the title.
    let section_idx = lines[title_idx + 1..]
        .iter()
        .position(|l| l.starts_with("## "))
        .map(|i| title_idx + 1 + i);

    let title_line = lines[title_idx];

    let mut result = String::new();

    // Everything before (and including) the title.
    for line in &lines[..=title_idx] {
        result.push_str(line);
        result.push('\n');
    }

    // New description (always preceded by a blank line).
    result.push('\n');
    result.push_str(new_desc);
    result.push('\n');

    // Rest of the body from the first ## heading onward.
    if let Some(si) = section_idx {
        result.push('\n');
        for line in &lines[si..] {
            result.push_str(line);
            result.push('\n');
        }
    }

    // Match original trailing-newline behaviour.
    if !body.ends_with('\n') {
        result = result.trim_end_matches('\n').to_string();
    }

    let _ = title_line; // suppress unused warning
    result
}

/// The canonical insertion order for sections.
const SECTION_ORDER: &[&str] = &["Design", "Acceptance Criteria", "Notes"];

/// Replace the content of a named `## <section_name>` section, or insert a
/// new one in canonical order if it is absent.
///
/// Canonical insertion order: Design < Acceptance Criteria < Notes.
///
/// If the section already exists, its content is replaced in-place.
/// If it does not exist, it is inserted before the first existing section
/// that should follow it in canonical order.  If no such section exists, it
/// is appended at the end of the body.
///
/// The section content always ends with a trailing newline.
pub fn replace_section(body: &str, section_name: &str, new_content: &str) -> String {
    let section_heading = format!("## {section_name}");

    // --- Try to replace an existing section --------------------------------

    let lines: Vec<&str> = body.lines().collect();

    // Find the line index of `## <section_name>`.
    if let Some(start_idx) = lines.iter().position(|l| *l == section_heading) {
        // Find where this section ends: the next `## ` line, or end of body.
        let end_idx = lines[start_idx + 1..]
            .iter()
            .position(|l| l.starts_with("## "))
            .map(|i| start_idx + 1 + i)
            .unwrap_or(lines.len());

        // Rebuild the body with the section content replaced.
        let mut result = String::new();

        for line in &lines[..start_idx] {
            result.push_str(line);
            result.push('\n');
        }

        result.push_str(&section_heading);
        result.push('\n');
        result.push('\n');
        result.push_str(new_content);
        if !new_content.ends_with('\n') {
            result.push('\n');
        }

        // Reattach subsequent sections.
        if end_idx < lines.len() {
            result.push('\n');
            for line in &lines[end_idx..] {
                result.push_str(line);
                result.push('\n');
            }
            // Trim double trailing newline that can appear if the original
            // last section ended with a blank line before EOF.
            if body.ends_with('\n') && result.ends_with("\n\n") {
                result.pop();
            }
        }

        return result;
    }

    // --- Insert a new section in canonical order ---------------------------

    // Determine which sections follow `section_name` in canonical order.
    let my_rank = SECTION_ORDER
        .iter()
        .position(|&s| s == section_name)
        .unwrap_or(usize::MAX);

    // Find the first existing section that has a higher canonical rank.
    let insert_before: Option<usize> = lines.iter().position(|l| {
        if let Some(name) = l.strip_prefix("## ") {
            let rank = SECTION_ORDER
                .iter()
                .position(|&s| s == name)
                .unwrap_or(usize::MAX);
            rank > my_rank
        } else {
            false
        }
    });

    let new_section_block = format!("\n## {section_name}\n\n{new_content}");
    let new_section_block = if new_content.ends_with('\n') {
        new_section_block
    } else {
        format!("{new_section_block}\n")
    };

    match insert_before {
        Some(idx) => {
            // Insert before lines[idx].
            let mut result = String::new();
            // Everything up to (not including) idx, but strip any trailing
            // blank lines so we can append a single blank separator.
            let before = lines[..idx].join("\n");
            let before = before.trim_end_matches('\n');
            result.push_str(before);
            result.push_str(&new_section_block);
            result.push('\n');
            for line in &lines[idx..] {
                result.push_str(line);
                result.push('\n');
            }
            result
        }
        None => {
            // Append at the end.
            let base = body.trim_end_matches('\n');
            format!("{base}{new_section_block}")
        }
    }
}

/// Apply tag mutations and return the resulting tags field value.
///
/// Exactly one of `tags` (replace), `add_tags` (merge), or `remove_tags`
/// (subtract) should be `Some`; if all are `None` the existing tags are
/// returned unchanged.
pub fn apply_tag_ops(
    existing: Option<Vec<String>>,
    tags: Option<&str>,
    add_tags: Option<&str>,
    remove_tags: Option<&str>,
) -> Option<Vec<String>> {
    if let Some(replace) = tags {
        // Replace all tags.
        let new_tags: Vec<String> = replace
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        if new_tags.is_empty() {
            None
        } else {
            Some(new_tags)
        }
    } else if let Some(add) = add_tags {
        // Merge, preserving existing order and deduplicating.
        let mut result: Vec<String> = existing.unwrap_or_default();
        for tag in add
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
        {
            if !result.contains(&tag) {
                result.push(tag);
            }
        }
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    } else if let Some(remove) = remove_tags {
        // Remove the specified tags.
        let to_remove: Vec<String> = remove
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        let result: Vec<String> = existing
            .unwrap_or_default()
            .into_iter()
            .filter(|t| !to_remove.contains(t))
            .collect();
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    } else {
        // No tag operation requested.
        existing
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_ticket_type(s: &str) -> Result<TicketType> {
    match s {
        "bug" => Ok(TicketType::Bug),
        "feature" => Ok(TicketType::Feature),
        "task" => Ok(TicketType::Task),
        "epic" => Ok(TicketType::Epic),
        "chore" => Ok(TicketType::Chore),
        other => Err(Error::InvalidType {
            value: other.to_string(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Write a ticket file into `<root>/.tickets/`.
    fn write_ticket(root: &Path, id: &str, content: &str) {
        let dir = root.join(".tickets");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(format!("{id}.md")), content).unwrap();
    }

    /// Read a ticket file back as a string.
    fn read_ticket(root: &Path, id: &str) -> String {
        fs::read_to_string(root.join(".tickets").join(format!("{id}.md"))).unwrap()
    }

    /// A minimal valid ticket body with all optional sections.
    fn full_body() -> &'static str {
        "# Original Title\n\nOriginal description.\n\n## Design\n\nDesign notes.\n\n## Acceptance Criteria\n\nMust pass.\n\n## Notes\n\nA note.\n"
    }

    /// A minimal ticket file content string.
    fn ticket_content(id: &str) -> String {
        format!(
            "---\nid: {id}\nstatus: open\ndeps: []\nlinks: []\ncreated: 2026-01-01T00:00:00Z\ntype: task\npriority: 2\nassignee: Alice\n---\n{}",
            full_body()
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn run_update(
        root: &Path,
        id: &str,
        title: Option<&str>,
        description: Option<&str>,
        design: Option<&str>,
        acceptance: Option<&str>,
        priority: Option<&str>,
        ticket_type: Option<&str>,
        assignee: Option<&str>,
        external_ref: Option<&str>,
        parent: Option<&str>,
        tags: Option<&str>,
        add_tags: Option<&str>,
        remove_tags: Option<&str>,
    ) -> Result<String> {
        update_impl(
            Some(root),
            id,
            title,
            description,
            design,
            acceptance,
            priority,
            ticket_type,
            assignee,
            external_ref,
            parent,
            tags,
            add_tags,
            remove_tags,
        )
    }

    // -----------------------------------------------------------------------
    // update_title — pure function
    // -----------------------------------------------------------------------

    #[test]
    fn update_title_replaces_heading() {
        let body = "# Old Title\n\nSome text.\n";
        let result = update_title(body, "New Title");
        assert!(
            result.contains("# New Title"),
            "new title missing\n{result}"
        );
        assert!(
            !result.contains("# Old Title"),
            "old title still present\n{result}"
        );
    }

    #[test]
    fn update_title_preserves_rest_of_body() {
        let body = "# Old Title\n\nSome text.\n\n## Notes\n\nA note.\n";
        let result = update_title(body, "New Title");
        assert!(result.contains("Some text."), "body text lost\n{result}");
        assert!(result.contains("## Notes"), "## Notes lost\n{result}");
        assert!(result.contains("A note."), "note text lost\n{result}");
    }

    #[test]
    fn update_title_no_heading_unchanged() {
        let body = "No heading here.\n";
        let result = update_title(body, "Title");
        assert_eq!(result, body);
    }

    // -----------------------------------------------------------------------
    // replace_description — pure function
    // -----------------------------------------------------------------------

    #[test]
    fn replace_description_new_text_present() {
        let body = "# T\n\nOld desc.\n\n## Notes\n\nNote.\n";
        let result = replace_description(body, "New description.");
        assert!(
            result.contains("New description."),
            "new desc missing\n{result}"
        );
    }

    #[test]
    fn replace_description_old_text_absent() {
        let body = "# T\n\nOld desc.\n\n## Notes\n\nNote.\n";
        let result = replace_description(body, "New description.");
        assert!(
            !result.contains("Old desc."),
            "old desc still present\n{result}"
        );
    }

    #[test]
    fn replace_description_sections_preserved() {
        let body = "# T\n\nOld desc.\n\n## Notes\n\nNote.\n";
        let result = replace_description(body, "New desc.");
        assert!(result.contains("## Notes"), "## Notes lost\n{result}");
        assert!(result.contains("Note."), "note text lost\n{result}");
    }

    #[test]
    fn replace_description_between_title_and_section() {
        let body = "# T\n\nOld.\n\n## Design\n\nDesign.\n";
        let result = replace_description(body, "New.");
        // New description must come before ## Design.
        let desc_pos = result.find("New.").unwrap();
        let section_pos = result.find("## Design").unwrap();
        assert!(
            desc_pos < section_pos,
            "description not before ## Design\n{result}"
        );
    }

    // -----------------------------------------------------------------------
    // replace_section — replace existing ## Design
    // -----------------------------------------------------------------------

    #[test]
    fn replace_section_replaces_existing_design() {
        let body = "# T\n\n## Design\n\nOld design.\n\n## Notes\n\nNote.\n";
        let result = replace_section(body, "Design", "New design.");
        assert!(
            result.contains("New design."),
            "new design missing\n{result}"
        );
        assert!(
            !result.contains("Old design."),
            "old design still present\n{result}"
        );
    }

    #[test]
    fn replace_section_preserves_other_sections_when_replacing() {
        let body = "# T\n\n## Design\n\nOld design.\n\n## Notes\n\nNote.\n";
        let result = replace_section(body, "Design", "New design.");
        assert!(result.contains("## Notes"), "## Notes lost\n{result}");
        assert!(result.contains("Note."), "note text lost\n{result}");
    }

    // -----------------------------------------------------------------------
    // replace_section — insert ## Design when absent
    // -----------------------------------------------------------------------

    #[test]
    fn replace_section_inserts_design_when_absent() {
        let body = "# T\n\nDesc.\n\n## Notes\n\nNote.\n";
        let result = replace_section(body, "Design", "New design.");
        assert!(
            result.contains("## Design"),
            "## Design not inserted\n{result}"
        );
        assert!(
            result.contains("New design."),
            "design text missing\n{result}"
        );
    }

    // -----------------------------------------------------------------------
    // replace_section — replace/insert ## Acceptance Criteria
    // -----------------------------------------------------------------------

    #[test]
    fn replace_section_replaces_acceptance_criteria() {
        let body = "# T\n\n## Acceptance Criteria\n\nOld criteria.\n";
        let result = replace_section(body, "Acceptance Criteria", "New criteria.");
        assert!(
            result.contains("New criteria."),
            "new criteria missing\n{result}"
        );
        assert!(
            !result.contains("Old criteria."),
            "old criteria still present\n{result}"
        );
    }

    #[test]
    fn replace_section_inserts_acceptance_criteria_when_absent() {
        let body = "# T\n\nDesc.\n\n## Notes\n\nNote.\n";
        let result = replace_section(body, "Acceptance Criteria", "Must pass.");
        assert!(
            result.contains("## Acceptance Criteria"),
            "## Acceptance Criteria not inserted\n{result}"
        );
        assert!(
            result.contains("Must pass."),
            "criteria text missing\n{result}"
        );
    }

    // -----------------------------------------------------------------------
    // Section insertion order
    // -----------------------------------------------------------------------

    #[test]
    fn section_insertion_order_design_before_acceptance_before_notes() {
        // Start with only ## Notes; insert both Design and Acceptance Criteria.
        let body = "# T\n\nDesc.\n\n## Notes\n\nNote.\n";
        let after_design = replace_section(body, "Design", "Design content.");
        let after_both = replace_section(&after_design, "Acceptance Criteria", "AC content.");

        let design_pos = after_both.find("## Design").unwrap();
        let ac_pos = after_both.find("## Acceptance Criteria").unwrap();
        let notes_pos = after_both.find("## Notes").unwrap();

        assert!(
            design_pos < ac_pos,
            "## Design should come before ## Acceptance Criteria\n{after_both}"
        );
        assert!(
            ac_pos < notes_pos,
            "## Acceptance Criteria should come before ## Notes\n{after_both}"
        );
    }

    // -----------------------------------------------------------------------
    // apply_tag_ops — pure function
    // -----------------------------------------------------------------------

    #[test]
    fn tags_replace_all() {
        let existing = Some(vec!["a".into(), "b".into()]);
        let result = apply_tag_ops(existing, Some("c,d"), None, None);
        assert_eq!(result, Some(vec!["c".to_string(), "d".to_string()]));
    }

    #[test]
    fn add_tags_merges_deduped() {
        let existing = Some(vec!["a".into(), "b".into()]);
        let result = apply_tag_ops(existing, None, Some("b,c"), None);
        assert_eq!(
            result,
            Some(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );
    }

    #[test]
    fn remove_tags_removes_specific() {
        let existing = Some(vec!["a".into(), "b".into(), "c".into()]);
        let result = apply_tag_ops(existing, None, None, Some("b"));
        assert_eq!(result, Some(vec!["a".to_string(), "c".to_string()]));
    }

    #[test]
    fn remove_tags_deletes_field_when_empty() {
        let existing = Some(vec!["only".into()]);
        let result = apply_tag_ops(existing, None, None, Some("only"));
        assert_eq!(
            result, None,
            "tags field should be absent when all tags removed"
        );
    }

    #[test]
    fn no_tag_op_preserves_existing() {
        let existing = Some(vec!["x".into()]);
        let result = apply_tag_ops(existing.clone(), None, None, None);
        assert_eq!(result, existing);
    }

    // -----------------------------------------------------------------------
    // Integration tests via update_impl
    // -----------------------------------------------------------------------

    #[test]
    fn update_priority() {
        let tmp = tempdir().unwrap();
        write_ticket(tmp.path(), "t-0001", &ticket_content("t-0001"));
        run_update(
            tmp.path(),
            "t-0001",
            None,
            None,
            None,
            None,
            Some("4"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let content = read_ticket(tmp.path(), "t-0001");
        assert!(
            content.contains("priority: 4"),
            "priority not updated\n{content}"
        );
    }

    #[test]
    fn update_type() {
        let tmp = tempdir().unwrap();
        write_ticket(tmp.path(), "t-0001", &ticket_content("t-0001"));
        run_update(
            tmp.path(),
            "t-0001",
            None,
            None,
            None,
            None,
            None,
            Some("bug"),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let content = read_ticket(tmp.path(), "t-0001");
        assert!(content.contains("type: bug"), "type not updated\n{content}");
    }

    #[test]
    fn update_assignee() {
        let tmp = tempdir().unwrap();
        write_ticket(tmp.path(), "t-0001", &ticket_content("t-0001"));
        run_update(
            tmp.path(),
            "t-0001",
            None,
            None,
            None,
            None,
            None,
            None,
            Some("Bob"),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let content = read_ticket(tmp.path(), "t-0001");
        assert!(
            content.contains("assignee: Bob"),
            "assignee not updated\n{content}"
        );
    }

    #[test]
    fn update_external_ref() {
        let tmp = tempdir().unwrap();
        write_ticket(tmp.path(), "t-0001", &ticket_content("t-0001"));
        run_update(
            tmp.path(),
            "t-0001",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("GH-99"),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let content = read_ticket(tmp.path(), "t-0001");
        assert!(
            content.contains("external-ref: GH-99"),
            "external-ref not updated\n{content}"
        );
    }

    #[test]
    fn update_parent_validation_nonexistent() {
        let tmp = tempdir().unwrap();
        write_ticket(tmp.path(), "t-0001", &ticket_content("t-0001"));
        let err = run_update(
            tmp.path(),
            "t-0001",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("nonexistent-9999"),
            None,
            None,
            None,
        )
        .unwrap_err();
        assert!(
            matches!(err, Error::TicketNotFound { .. }),
            "expected TicketNotFound for nonexistent parent, got {err:?}"
        );
    }

    #[test]
    fn update_tags_replace_all_integration() {
        let tmp = tempdir().unwrap();
        let content = format!(
            "---\nid: t-0001\nstatus: open\ndeps: []\nlinks: []\ncreated: 2026-01-01T00:00:00Z\ntype: task\npriority: 2\ntags: [a, b]\n---\n# T\n"
        );
        write_ticket(tmp.path(), "t-0001", &content);
        run_update(
            tmp.path(),
            "t-0001",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("c,d"),
            None,
            None,
        )
        .unwrap();
        let result = read_ticket(tmp.path(), "t-0001");
        assert!(
            result.contains("tags: [c, d]"),
            "tags not replaced\n{result}"
        );
        assert!(
            !result.contains("[a, b]"),
            "old tags still present\n{result}"
        );
    }

    #[test]
    fn update_add_tags_merges_deduped_integration() {
        let tmp = tempdir().unwrap();
        let content = format!(
            "---\nid: t-0001\nstatus: open\ndeps: []\nlinks: []\ncreated: 2026-01-01T00:00:00Z\ntype: task\npriority: 2\ntags: [a, b]\n---\n# T\n"
        );
        write_ticket(tmp.path(), "t-0001", &content);
        run_update(
            tmp.path(),
            "t-0001",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("b,c"),
            None,
        )
        .unwrap();
        let result = read_ticket(tmp.path(), "t-0001");
        assert!(
            result.contains("tags: [a, b, c]"),
            "tags not merged correctly\n{result}"
        );
    }

    #[test]
    fn update_remove_tags_removes_specific_integration() {
        let tmp = tempdir().unwrap();
        let content = format!(
            "---\nid: t-0001\nstatus: open\ndeps: []\nlinks: []\ncreated: 2026-01-01T00:00:00Z\ntype: task\npriority: 2\ntags: [a, b, c]\n---\n# T\n"
        );
        write_ticket(tmp.path(), "t-0001", &content);
        run_update(
            tmp.path(),
            "t-0001",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("b"),
        )
        .unwrap();
        let result = read_ticket(tmp.path(), "t-0001");
        assert!(
            result.contains("tags: [a, c]"),
            "tags not updated\n{result}"
        );
    }

    #[test]
    fn update_remove_tags_deletes_field_when_empty_integration() {
        let tmp = tempdir().unwrap();
        let content = format!(
            "---\nid: t-0001\nstatus: open\ndeps: []\nlinks: []\ncreated: 2026-01-01T00:00:00Z\ntype: task\npriority: 2\ntags: [only]\n---\n# T\n"
        );
        write_ticket(tmp.path(), "t-0001", &content);
        run_update(
            tmp.path(),
            "t-0001",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("only"),
        )
        .unwrap();
        let result = read_ticket(tmp.path(), "t-0001");
        assert!(
            !result.contains("tags:"),
            "tags field should be absent after removing all tags\n{result}"
        );
    }

    #[test]
    fn unmodified_fields_preserved() {
        let tmp = tempdir().unwrap();
        let original = ticket_content("t-0001");
        write_ticket(tmp.path(), "t-0001", &original);

        // Update only the title; every byte outside the title line must be
        // identical to the original.
        run_update(
            tmp.path(),
            "t-0001",
            Some("New Title"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let result = read_ticket(tmp.path(), "t-0001");

        // Build the exact expected output: swap only the title heading line.
        let expected = original.replace("# Original Title\n", "# New Title\n");
        assert_eq!(
            result, expected,
            "result differed from expected byte-for-byte output"
        );
    }

    #[test]
    fn output_contains_ticket_id() {
        let tmp = tempdir().unwrap();
        write_ticket(tmp.path(), "t-0001", &ticket_content("t-0001"));
        let id = run_update(
            tmp.path(),
            "t-0001",
            Some("New Title"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(id, "t-0001", "returned ID should be the full ticket ID");
    }

    /// Verify that the public `update_with_writer` path actually writes the
    /// ticket ID to the provided writer (i.e. that the `println!` equivalent
    /// fires and prints the correct value).
    #[test]
    fn stdout_contains_ticket_id() {
        let tmp = tempdir().unwrap();
        write_ticket(tmp.path(), "t-0001", &ticket_content("t-0001"));

        let mut buf: Vec<u8> = Vec::new();
        update_with_writer(
            &mut buf,
            Some(tmp.path()),
            "t-0001",
            Some("Stdout Test"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let output = String::from_utf8(buf).unwrap();
        assert_eq!(
            output.trim_end_matches('\n'),
            "t-0001",
            "stdout should contain exactly the ticket ID\nGot: {output:?}"
        );
    }

    #[test]
    fn partial_id_resolution() {
        let tmp = tempdir().unwrap();
        write_ticket(tmp.path(), "t-9999", &ticket_content("t-9999"));
        // Resolve by suffix only.
        let id = run_update(
            tmp.path(),
            "9999",
            Some("Resolved Title"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(id, "t-9999");
        let content = read_ticket(tmp.path(), "t-9999");
        assert!(
            content.contains("# Resolved Title"),
            "title not updated\n{content}"
        );
    }

    // -----------------------------------------------------------------------
    // --tags mutual exclusivity with --add-tags and --remove-tags
    //
    // These exercise the clap-layer conflict rules declared in cli.rs.
    // -----------------------------------------------------------------------

    #[test]
    fn tags_and_add_tags_are_mutually_exclusive() {
        use crate::cli::Cli;
        use clap::Parser;

        let result = Cli::try_parse_from([
            "ticket",
            "update",
            "t-0001",
            "--tags",
            "a,b",
            "--add-tags",
            "c",
        ]);
        assert!(
            result.is_err(),
            "--tags and --add-tags should be mutually exclusive but clap accepted both"
        );
    }

    #[test]
    fn tags_and_remove_tags_are_mutually_exclusive() {
        use crate::cli::Cli;
        use clap::Parser;

        let result = Cli::try_parse_from([
            "ticket",
            "update",
            "t-0001",
            "--tags",
            "a,b",
            "--remove-tags",
            "a",
        ]);
        assert!(
            result.is_err(),
            "--tags and --remove-tags should be mutually exclusive but clap accepted both"
        );
    }

    // -----------------------------------------------------------------------
    // Verify that non-targeted frontmatter fields (e.g. status) are not
    // corrupted by a partial update.
    // -----------------------------------------------------------------------
    #[test]
    fn status_roundtrip_via_update() {
        let tmp = tempdir().unwrap();
        write_ticket(tmp.path(), "t-0001", &ticket_content("t-0001"));
        // Update only priority; verify status is still open (not corrupted).
        run_update(
            tmp.path(),
            "t-0001",
            None,
            None,
            None,
            None,
            Some("3"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let content = read_ticket(tmp.path(), "t-0001");
        assert!(
            content.contains("status: open"),
            "status corrupted\n{content}"
        );
        assert!(
            content.contains("priority: 3"),
            "priority not updated\n{content}"
        );
    }
}
