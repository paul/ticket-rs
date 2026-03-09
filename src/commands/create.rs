// Implementation of the `create` subcommand.

use std::path::Path;

use chrono::Utc;

use crate::error::{Error, Result};
use crate::id::generate_id;
use crate::store::TicketStore;
use crate::ticket::{Status, Ticket, TicketType};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Create a new ticket and print its ID to stdout.
///
/// Resolves the default assignee from `git config user.name` when none is
/// provided via `--assignee`. Delegates the actual work to [`create_impl`].
#[allow(clippy::too_many_arguments)]
pub fn create(
    title: &str,
    description: Option<&str>,
    design: Option<&str>,
    acceptance: Option<&str>,
    ticket_type: &str,
    priority: &str,
    assignee: Option<&str>,
    external_ref: Option<&str>,
    parent: Option<&str>,
    tags: Option<&str>,
) -> Result<()> {
    // Resolve default assignee from git config when the flag was not supplied.
    let git_name: Option<String> = if assignee.is_none() {
        git2::Config::open_default()
            .and_then(|c| c.get_string("user.name"))
            .ok()
    } else {
        None
    };
    let resolved_assignee = assignee.or(git_name.as_deref());

    let id = create_impl(
        None,
        title,
        description,
        design,
        acceptance,
        ticket_type,
        priority,
        resolved_assignee,
        external_ref,
        parent,
        tags,
        generate_id,
    )?;

    println!("{id}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal implementation (testable via explicit start_dir)
// ---------------------------------------------------------------------------

/// Core logic for ticket creation.
///
/// `start_dir` is the directory from which to locate (or create) `.tickets/`.
/// Passing `None` uses the current working directory, which is what the public
/// [`create`] function does. Tests pass `Some(tempdir.path())` to avoid
/// touching the real filesystem.
///
/// `id_gen` generates a candidate ID from a directory name. Production code
/// passes [`generate_id`]; tests may inject a deterministic stub to force
/// collision scenarios.
///
/// Returns the ID of the newly created ticket.
#[allow(clippy::too_many_arguments)]
fn create_impl(
    start_dir: Option<&Path>,
    title: &str,
    description: Option<&str>,
    design: Option<&str>,
    acceptance: Option<&str>,
    ticket_type: &str,
    priority: &str,
    assignee: Option<&str>,
    external_ref: Option<&str>,
    parent: Option<&str>,
    tags: Option<&str>,
    id_gen: impl Fn(&str) -> String,
) -> Result<String> {
    // --- Parse and validate inputs ------------------------------------------

    let ticket_type = parse_ticket_type(ticket_type)?;
    let priority = parse_priority(priority)?;
    let tags = parse_tags(tags);

    // --- Locate / create the store ------------------------------------------

    let store = TicketStore::ensure(start_dir)?;

    // --- Validate parent (must resolve to an existing ticket) ---------------

    let parent_id: Option<String> = match parent {
        None => None,
        Some(partial) => {
            let path = store.resolve_id(partial)?;
            // Extract the full ID from the filename stem.
            let stem =
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| Error::TicketNotFound {
                        id: partial.to_string(),
                    })?;
            Some(stem.to_string())
        }
    };

    // --- Derive the ID prefix from the project directory --------------------

    let dir_name = store
        .dir()
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty())
        .ok_or_else(|| {
            Error::Io(std::io::Error::other(
                "unable to determine ticket prefix from path: .tickets/ has no parent directory name",
            ))
        })?;

    // --- Generate a collision-free ID ---------------------------------------

    let id = loop {
        let candidate = id_gen(dir_name);
        if !store.dir().join(format!("{candidate}.md")).exists() {
            break candidate;
        }
    };

    // --- Build the markdown body --------------------------------------------

    let body = build_body(title, description, design, acceptance);

    // --- Construct and write the ticket -------------------------------------

    let ticket = Ticket {
        id: id.clone(),
        status: Status::Open,
        deps: vec![],
        links: vec![],
        created: Utc::now(),
        ticket_type,
        priority,
        assignee: assignee.map(str::to_string),
        external_ref: external_ref.map(str::to_string),
        parent: parent_id,
        tags,
        title: title.to_string(),
        body,
    };

    store.write_ticket(&ticket)?;

    Ok(id)
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

fn parse_priority(s: &str) -> Result<u8> {
    let n: u8 = s.parse().map_err(|_| Error::InvalidPriority {
        value: s.to_string(),
    })?;
    if n > 4 {
        return Err(Error::InvalidPriority {
            value: s.to_string(),
        });
    }
    Ok(n)
}

fn parse_tags(s: Option<&str>) -> Option<Vec<String>> {
    let s = s?;
    let tags: Vec<String> = s
        .split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();
    if tags.is_empty() { None } else { Some(tags) }
}

/// Build the markdown body for a new ticket.
///
/// Structure:
/// ```text
/// # {title}
///
/// {description}          ← only if provided
///
/// ## Design              ← only if provided
///
/// {design}
///
/// ## Acceptance Criteria ← only if provided
///
/// {acceptance}
/// ```
/// The body always ends with a single trailing newline.
fn build_body(
    title: &str,
    description: Option<&str>,
    design: Option<&str>,
    acceptance: Option<&str>,
) -> String {
    let mut body = format!("# {title}\n");

    if let Some(desc) = description {
        body.push('\n');
        body.push_str(desc);
        body.push('\n');
    }

    if let Some(design) = design {
        body.push_str("\n## Design\n\n");
        body.push_str(design);
        body.push('\n');
    }

    if let Some(acc) = acceptance {
        body.push_str("\n## Acceptance Criteria\n\n");
        body.push_str(acc);
        body.push('\n');
    }

    body
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // -----------------------------------------------------------------------
    // Helper: run create_impl against a fresh temp directory.
    // -----------------------------------------------------------------------

    /// Calls create_impl with the given start_dir (which should be the root of
    /// a temp directory). Returns the generated ID and leaves the .tickets/
    /// directory in place for inspection.
    fn run_create(
        root: &TempDir,
        title: &str,
        description: Option<&str>,
        design: Option<&str>,
        acceptance: Option<&str>,
        ticket_type: &str,
        priority: &str,
        assignee: Option<&str>,
        external_ref: Option<&str>,
        parent: Option<&str>,
        tags: Option<&str>,
    ) -> Result<String> {
        create_impl(
            Some(root.path()),
            title,
            description,
            design,
            acceptance,
            ticket_type,
            priority,
            assignee,
            external_ref,
            parent,
            tags,
            generate_id,
        )
    }

    /// Read the ticket file for `id` from root/.tickets/.
    fn read_ticket_file(root: &TempDir, id: &str) -> String {
        std::fs::read_to_string(root.path().join(".tickets").join(format!("{id}.md")))
            .expect("ticket file should exist")
    }

    // -----------------------------------------------------------------------
    // Default field values
    // -----------------------------------------------------------------------

    #[test]
    fn default_field_values() {
        let root = tempfile::tempdir().unwrap();
        let id = run_create(
            &root,
            "My ticket",
            None,
            None,
            None,
            "task",
            "2",
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let content = read_ticket_file(&root, &id);

        assert!(
            content.contains("status: open"),
            "status should be open\n{content}"
        );
        assert!(
            content.contains("priority: 2"),
            "priority should be 2\n{content}"
        );
        assert!(
            content.contains("type: task"),
            "type should be task\n{content}"
        );
        assert!(
            content.contains("deps: []"),
            "deps should be empty\n{content}"
        );
        assert!(
            content.contains("links: []"),
            "links should be empty\n{content}"
        );
    }

    // -----------------------------------------------------------------------
    // Title set
    // -----------------------------------------------------------------------

    #[test]
    fn title_set() {
        let root = tempfile::tempdir().unwrap();
        let id = run_create(
            &root,
            "My Ticket Title",
            None,
            None,
            None,
            "task",
            "2",
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let content = read_ticket_file(&root, &id);
        assert!(
            content.contains("# My Ticket Title"),
            "title heading not found\n{content}"
        );
    }

    // -----------------------------------------------------------------------
    // Default title (Untitled)
    // -----------------------------------------------------------------------

    #[test]
    fn default_title_untitled() {
        // main.rs already defaults the title to "Untitled" before calling create,
        // so we mirror that here.
        let root = tempfile::tempdir().unwrap();
        let id = run_create(
            &root, "Untitled", None, None, None, "task", "2", None, None, None, None,
        )
        .unwrap();
        let content = read_ticket_file(&root, &id);
        assert!(
            content.contains("# Untitled"),
            "expected '# Untitled' heading\n{content}"
        );
    }

    // -----------------------------------------------------------------------
    // -d description
    // -----------------------------------------------------------------------

    #[test]
    fn description_appears_in_body() {
        let root = tempfile::tempdir().unwrap();
        let id = run_create(
            &root,
            "T",
            Some("This is the description"),
            None,
            None,
            "task",
            "2",
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let content = read_ticket_file(&root, &id);

        // Description must appear after the title heading and before any ## heading.
        let after_title = content
            .split("# T\n")
            .nth(1)
            .expect("title heading not found");
        let before_next_section = after_title.split("\n##").next().unwrap_or(after_title);
        assert!(
            before_next_section.contains("This is the description"),
            "description not found between title and next section\n{content}"
        );
    }

    // -----------------------------------------------------------------------
    // --design section
    // -----------------------------------------------------------------------

    #[test]
    fn design_section_created() {
        let root = tempfile::tempdir().unwrap();
        let id = run_create(
            &root,
            "T",
            None,
            Some("Use microservices"),
            None,
            "task",
            "2",
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let content = read_ticket_file(&root, &id);
        assert!(
            content.contains("## Design"),
            "## Design section missing\n{content}"
        );
        assert!(
            content.contains("Use microservices"),
            "design text missing\n{content}"
        );
    }

    // -----------------------------------------------------------------------
    // --acceptance section
    // -----------------------------------------------------------------------

    #[test]
    fn acceptance_section_created() {
        let root = tempfile::tempdir().unwrap();
        let id = run_create(
            &root,
            "T",
            None,
            None,
            Some("Should pass all tests"),
            "task",
            "2",
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let content = read_ticket_file(&root, &id);
        assert!(
            content.contains("## Acceptance Criteria"),
            "## Acceptance Criteria missing\n{content}"
        );
        assert!(
            content.contains("Should pass all tests"),
            "acceptance text missing\n{content}"
        );
    }

    // -----------------------------------------------------------------------
    // -t type
    // -----------------------------------------------------------------------

    #[test]
    fn type_flag() {
        let root = tempfile::tempdir().unwrap();
        let id = run_create(
            &root, "T", None, None, None, "bug", "2", None, None, None, None,
        )
        .unwrap();
        let content = read_ticket_file(&root, &id);
        assert!(
            content.contains("type: bug"),
            "expected type: bug\n{content}"
        );
    }

    // -----------------------------------------------------------------------
    // -p priority
    // -----------------------------------------------------------------------

    #[test]
    fn priority_flag() {
        let root = tempfile::tempdir().unwrap();
        let id = run_create(
            &root, "T", None, None, None, "task", "0", None, None, None, None,
        )
        .unwrap();
        let content = read_ticket_file(&root, &id);
        assert!(
            content.contains("priority: 0"),
            "expected priority: 0\n{content}"
        );
    }

    // -----------------------------------------------------------------------
    // -a assignee
    // -----------------------------------------------------------------------

    #[test]
    fn assignee_flag() {
        let root = tempfile::tempdir().unwrap();
        let id = run_create(
            &root,
            "T",
            None,
            None,
            None,
            "task",
            "2",
            Some("Jane"),
            None,
            None,
            None,
        )
        .unwrap();
        let content = read_ticket_file(&root, &id);
        assert!(
            content.contains("assignee: Jane"),
            "expected assignee: Jane\n{content}"
        );
    }

    // -----------------------------------------------------------------------
    // --external-ref
    // -----------------------------------------------------------------------

    #[test]
    fn external_ref_flag() {
        let root = tempfile::tempdir().unwrap();
        let id = run_create(
            &root,
            "T",
            None,
            None,
            None,
            "task",
            "2",
            None,
            Some("JIRA-42"),
            None,
            None,
        )
        .unwrap();
        let content = read_ticket_file(&root, &id);
        assert!(
            content.contains("external-ref: JIRA-42"),
            "expected external-ref: JIRA-42\n{content}"
        );
    }

    // -----------------------------------------------------------------------
    // --parent validation: nonexistent parent returns error
    // -----------------------------------------------------------------------

    #[test]
    fn parent_validation_error() {
        let root = tempfile::tempdir().unwrap();
        // .tickets/ must exist for the store to find it, but the parent ticket doesn't.
        std::fs::create_dir_all(root.path().join(".tickets")).unwrap();
        let result = run_create(
            &root,
            "T",
            None,
            None,
            None,
            "task",
            "2",
            None,
            None,
            Some("nonexistent"),
            None,
        );
        assert!(result.is_err(), "expected error for nonexistent parent");
    }

    // -----------------------------------------------------------------------
    // --tags
    // -----------------------------------------------------------------------

    #[test]
    fn tags_flag() {
        let root = tempfile::tempdir().unwrap();
        let id = run_create(
            &root,
            "T",
            None,
            None,
            None,
            "task",
            "2",
            None,
            None,
            None,
            Some("ui,backend"),
        )
        .unwrap();
        let content = read_ticket_file(&root, &id);
        assert!(
            content.contains("tags: [ui, backend]"),
            "expected tags: [ui, backend]\n{content}"
        );
    }

    // -----------------------------------------------------------------------
    // created timestamp
    // -----------------------------------------------------------------------

    #[test]
    fn created_timestamp_present_and_valid() {
        let root = tempfile::tempdir().unwrap();
        let id = run_create(
            &root, "T", None, None, None, "task", "2", None, None, None, None,
        )
        .unwrap();
        let content = read_ticket_file(&root, &id);

        // Must match the ISO 8601 UTC format used by write_to_string.
        let re = regex_lite_datetime(&content);
        assert!(re, "no valid created timestamp found\n{content}");
    }

    /// Check that the content contains a line matching `created: YYYY-MM-DDTHH:MM:SSZ`.
    fn regex_lite_datetime(content: &str) -> bool {
        content.lines().any(|line| {
            if let Some(rest) = line.strip_prefix("created: ") {
                // Basic structure check: 20 chars, ends with Z, has T and hyphens/colons.
                rest.len() == 20
                    && rest.ends_with('Z')
                    && rest.chars().nth(10) == Some('T')
                    && rest[..10].contains('-')
            } else {
                false
            }
        })
    }

    // -----------------------------------------------------------------------
    // ID format (matches pattern)
    // -----------------------------------------------------------------------

    #[test]
    fn id_format_matches_pattern() {
        // tempfile::tempdir() names start with '.' which produce non-alpha prefixes,
        // so we create a named subdirectory with a clean project name.
        let root = tempfile::tempdir().unwrap();
        let project_dir = root.path().join("my-project");
        std::fs::create_dir_all(&project_dir).unwrap();
        // Use the project dir itself as the start dir so the .tickets/ dir ends
        // up inside it with the right parent name.
        let id = create_impl(
            Some(&project_dir),
            "T",
            None,
            None,
            None,
            "task",
            "2",
            None,
            None,
            None,
            None,
            generate_id,
        )
        .unwrap();

        // ID must be PREFIX-SUFFIX where prefix is lowercase alpha (2-4 chars)
        // and suffix is 4 lowercase hex chars.
        let Some((prefix, suffix)) = id.split_once('-') else {
            panic!("ID '{id}' has no '-' separator");
        };
        assert!(
            (2..=4).contains(&prefix.len()) && prefix.chars().all(|c| c.is_ascii_lowercase()),
            "prefix '{prefix}' should be 2-4 lowercase alpha chars"
        );
        assert_eq!(suffix.len(), 4, "suffix '{suffix}' should be 4 chars");
        assert!(
            suffix.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')),
            "suffix '{suffix}' should be lowercase hex"
        );
    }

    // -----------------------------------------------------------------------
    // ID collision retry
    // -----------------------------------------------------------------------

    #[test]
    fn id_collision_retry() {
        use std::cell::Cell;

        let root = tempfile::tempdir().unwrap();
        let tickets_dir = root.path().join(".tickets");
        std::fs::create_dir_all(&tickets_dir).unwrap();

        // Pre-seed the colliding ID before calling create_impl.
        let colliding_id = "tmp-aaaa";
        std::fs::write(
            tickets_dir.join(format!("{colliding_id}.md")),
            b"placeholder",
        )
        .unwrap();

        // Inject a deterministic generator that returns the colliding ID on the
        // first call and a different, guaranteed-free ID on the second.
        let call_count = Cell::new(0u32);
        let id_gen = |_dir: &str| {
            let n = call_count.get();
            call_count.set(n + 1);
            if n == 0 {
                colliding_id.to_string() // first call: guaranteed to collide
            } else {
                "tmp-bbbb".to_string() // second call: guaranteed to be free
            }
        };

        let id = create_impl(
            Some(root.path()),
            "T",
            None,
            None,
            None,
            "task",
            "2",
            None,
            None,
            None,
            None,
            id_gen,
        )
        .unwrap();

        // The retry must have fired: the first candidate collided, so we got the second.
        assert_eq!(
            call_count.get(),
            2,
            "id_gen should have been called twice (collision + retry)"
        );
        assert_eq!(
            id, "tmp-bbbb",
            "should have used the second (non-colliding) ID"
        );
        assert!(
            tickets_dir.join("tmp-bbbb.md").exists(),
            "ticket file should exist for the retried ID"
        );
        assert!(
            tickets_dir.join("tmp-aaaa.md").exists(),
            "pre-seeded placeholder should still exist"
        );
    }
}
