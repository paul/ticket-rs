// Input resolution for the `@` convention used by text-heavy CLI fields.
//
// The following prefix rules apply to any supported field value:
//
//   `@@literal`  → the literal string `@literal` (escape: strip one leading `@`)
//   `@-` or `-`  → read entire stdin, strip at most one trailing newline
//   `@path`      → read the file at `path`, strip at most one trailing newline
//   anything else → return unchanged (passthrough)
//
// Only one field per invocation may read from stdin.  Call
// `validate_no_multiple_stdin` with all raw field values before resolving any
// of them; it returns an error if more than one would trigger a stdin read.

use std::fs;
use std::io::{self, Read};

use crate::error::{Error, Result};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Resolve a CLI field value according to the `@` input convention.
///
/// Returns the resolved string on success, or an `Error` if a referenced file
/// cannot be read.
pub fn resolve_input(value: &str) -> Result<String> {
    if let Some(rest) = value.strip_prefix("@@") {
        // `@@…` → literal `@…` (strip one leading `@`)
        return Ok(format!("@{rest}"));
    }

    if value == "-" || value == "@-" {
        // Read entire stdin, trim at most one trailing newline (shell convention).
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf).map_err(Error::Io)?;
        return Ok(buf.strip_suffix('\n').unwrap_or(&buf).to_string());
    }

    if let Some(path) = value.strip_prefix('@') {
        // `@path` → read file contents, trim at most one trailing newline.
        let contents = fs::read_to_string(path).map_err(|source| Error::InputFileError {
            path: path.to_string(),
            source,
        })?;
        return Ok(contents.strip_suffix('\n').unwrap_or(&contents).to_string());
    }

    // Passthrough: return the value unchanged.
    Ok(value.to_string())
}

/// Return an error if more than one of the provided raw values would trigger a
/// stdin read (i.e. is `"-"` or `"@-"`).
///
/// Pass `None` for fields that were not supplied on the command line.
pub fn validate_no_multiple_stdin(values: &[Option<&str>]) -> Result<()> {
    let stdin_count = values
        .iter()
        .filter(|v| matches!(*v, Some("-") | Some("@-")))
        .count();

    if stdin_count > 1 {
        return Err(Error::MultipleStdin);
    }
    Ok(())
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
    // resolve_input — passthrough
    // -----------------------------------------------------------------------

    #[test]
    fn passthrough_plain_string() {
        let result = resolve_input("hello world").unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn passthrough_empty_string() {
        let result = resolve_input("").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn passthrough_string_with_special_chars() {
        let result = resolve_input("foo `bar` \"baz\"").unwrap();
        assert_eq!(result, "foo `bar` \"baz\"");
    }

    // -----------------------------------------------------------------------
    // resolve_input — @@ escape
    // -----------------------------------------------------------------------

    #[test]
    fn double_at_returns_literal_at_prefix() {
        let result = resolve_input("@@github").unwrap();
        assert_eq!(result, "@github");
    }

    #[test]
    fn double_at_with_empty_rest() {
        // `@@` → literal `@`
        let result = resolve_input("@@").unwrap();
        assert_eq!(result, "@");
    }

    #[test]
    fn double_at_only_strips_one_at() {
        // `@@@foo` → `@@foo` (strip exactly one leading `@`)
        let result = resolve_input("@@@foo").unwrap();
        assert_eq!(result, "@@foo");
    }

    // -----------------------------------------------------------------------
    // resolve_input — @path file reading
    // -----------------------------------------------------------------------

    #[test]
    fn at_path_reads_file_contents() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("desc.md");
        fs::write(&file, "File content here").unwrap();

        let path_str = format!("@{}", file.display());
        let result = resolve_input(&path_str).unwrap();
        assert_eq!(result, "File content here");
    }

    #[test]
    fn at_path_strips_single_trailing_newline() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("desc.md");
        fs::write(&file, "Content\n").unwrap();

        let path_str = format!("@{}", file.display());
        let result = resolve_input(&path_str).unwrap();
        assert_eq!(result, "Content");
    }

    #[test]
    fn at_path_preserves_multiple_trailing_newlines() {
        // Only the single outermost newline is stripped; additional trailing
        // newlines are intentional content and must be preserved.
        let dir = tempdir().unwrap();
        let file = dir.path().join("desc.md");
        fs::write(&file, "Content\n\n").unwrap();

        let path_str = format!("@{}", file.display());
        let result = resolve_input(&path_str).unwrap();
        assert_eq!(result, "Content\n");
    }

    #[test]
    fn at_path_no_trailing_newline_unchanged() {
        // Files without a trailing newline are returned as-is.
        let dir = tempdir().unwrap();
        let file = dir.path().join("desc.md");
        fs::write(&file, "Content").unwrap();

        let path_str = format!("@{}", file.display());
        let result = resolve_input(&path_str).unwrap();
        assert_eq!(result, "Content");
    }

    #[test]
    fn at_path_preserves_internal_newlines() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("multi.md");
        fs::write(&file, "Line one\nLine two\n").unwrap();

        let path_str = format!("@{}", file.display());
        let result = resolve_input(&path_str).unwrap();
        assert_eq!(result, "Line one\nLine two");
    }

    #[test]
    fn at_path_file_not_found_returns_error() {
        let result = resolve_input("@/nonexistent/path/to/file.md");
        assert!(result.is_err(), "expected error for missing file");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("cannot read"),
            "expected 'cannot read' in error, got: {msg}"
        );
        assert!(
            msg.contains("/nonexistent/path/to/file.md"),
            "expected path in error message, got: {msg}"
        );
    }

    #[test]
    fn at_path_error_message_includes_at_sign() {
        let result = resolve_input("@/no/such/file.md");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("@/no/such/file.md"),
            "expected '@path' in error message, got: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // validate_no_multiple_stdin
    // -----------------------------------------------------------------------

    #[test]
    fn no_stdin_values_is_ok() {
        let result = validate_no_multiple_stdin(&[Some("foo"), Some("bar"), None]);
        assert!(result.is_ok());
    }

    #[test]
    fn single_dash_is_ok() {
        let result = validate_no_multiple_stdin(&[Some("-"), Some("foo"), None]);
        assert!(result.is_ok());
    }

    #[test]
    fn single_at_dash_is_ok() {
        let result = validate_no_multiple_stdin(&[Some("@-"), None, None]);
        assert!(result.is_ok());
    }

    #[test]
    fn two_dash_values_returns_error() {
        let result = validate_no_multiple_stdin(&[Some("-"), Some("-"), None]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::MultipleStdin));
    }

    #[test]
    fn two_at_dash_values_returns_error() {
        let result = validate_no_multiple_stdin(&[Some("@-"), Some("@-"), None]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::MultipleStdin));
    }

    #[test]
    fn mixed_dash_and_at_dash_returns_error() {
        let result = validate_no_multiple_stdin(&[Some("-"), Some("@-"), None]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::MultipleStdin));
    }

    #[test]
    fn none_values_are_ignored() {
        let result = validate_no_multiple_stdin(&[None, None, None]);
        assert!(result.is_ok());
    }

    #[test]
    fn error_message_is_clear() {
        let result = validate_no_multiple_stdin(&[Some("-"), Some("-")]);
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("only one field may read from stdin at a time"),
            "expected clear error message, got: {msg}"
        );
    }
}
