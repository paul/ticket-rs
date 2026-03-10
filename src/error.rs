// Error types for the ticket library.

use std::fmt;

use crate::ticket::Ticket;

/// Unified error type for all ticket library operations.
#[derive(Debug)]
pub enum Error {
    /// A ticket with the given ID could not be found.
    TicketNotFound {
        id: String,
        /// Up to 3 similar tickets to show as did-you-mean suggestions.
        suggestions: Vec<Ticket>,
    },
    /// A partial ID matched more than one ticket.
    AmbiguousId {
        partial: String,
        candidates: Vec<String>,
    },
    /// No `.tickets` directory could be found in the current or any parent directory.
    TicketsNotFound,
    /// An unrecognized status string was encountered.
    InvalidStatus {
        value: String,
        /// The closest known status value, if any.
        suggestion: Option<String>,
    },
    /// An unrecognized ticket type string was encountered.
    InvalidType {
        value: String,
        /// The closest known type value, if any.
        suggestion: Option<String>,
    },
    /// An unrecognized priority value was encountered.
    InvalidPriority { value: String },
    /// A dependency that was expected to exist was not found.
    DependencyNotFound,
    /// A link that was expected to exist was not found.
    LinkNotFound,
    /// The editor process exited with a non-zero status code.
    EditorError { editor: String, code: Option<i32> },
    /// An underlying I/O error.
    Io(std::io::Error),
    /// A YAML parse or serialization error.
    Yaml(serde_yaml::Error),
}

/// Convenience alias for `Result<T, Error>`.
pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::TicketNotFound { id, .. } => {
                write!(f, "ticket '{id}' not found")
            }
            Error::AmbiguousId {
                partial,
                candidates,
            } => {
                write!(
                    f,
                    "ambiguous id '{partial}', matches: {}",
                    candidates.join(", ")
                )
            }
            Error::TicketsNotFound => {
                write!(f, "no .tickets directory found")
            }
            Error::InvalidStatus { value, suggestion } => {
                write!(
                    f,
                    "invalid status '{value}', valid options: open in_progress closed"
                )?;
                if let Some(s) = suggestion {
                    write!(f, ", did you mean: {s}?")?;
                }
                Ok(())
            }
            Error::InvalidType { value, suggestion } => {
                write!(
                    f,
                    "invalid type '{value}', valid options: bug, feature, task, epic, chore"
                )?;
                if let Some(s) = suggestion {
                    write!(f, ", did you mean: {s}?")?;
                }
                Ok(())
            }
            Error::InvalidPriority { value } => {
                write!(
                    f,
                    "invalid priority '{value}', valid options: 0, 1, 2, 3, 4"
                )
            }
            Error::DependencyNotFound => write!(f, "Dependency not found"),
            Error::LinkNotFound => write!(f, "Link not found"),
            Error::EditorError { editor, code } => match code {
                Some(n) => write!(f, "editor '{editor}' exited with status {n}"),
                None => write!(f, "editor '{editor}' was terminated by a signal"),
            },
            Error::Io(err) => write!(f, "{err}"),
            Error::Yaml(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            Error::Yaml(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(err: serde_yaml::Error) -> Self {
        Error::Yaml(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as StdError;

    #[test]
    fn display_ticket_not_found_contains_id() {
        let err = Error::TicketNotFound {
            id: "abc-1234".to_string(),
            suggestions: vec![],
        };
        let msg = err.to_string();
        assert!(
            msg.contains("abc-1234"),
            "expected message to contain 'abc-1234', got: {msg}"
        );
        assert!(
            msg.contains("not found"),
            "expected message to contain 'not found', got: {msg}"
        );
    }

    #[test]
    fn display_ambiguous_id_contains_partial_and_candidates() {
        let err = Error::AmbiguousId {
            partial: "tr-ab".to_string(),
            candidates: vec!["tr-ab12".to_string(), "tr-ab34".to_string()],
        };
        let msg = err.to_string();
        assert!(
            msg.contains("tr-ab"),
            "expected message to contain partial id 'tr-ab', got: {msg}"
        );
        assert!(
            msg.contains("tr-ab12"),
            "expected message to list candidate 'tr-ab12', got: {msg}"
        );
        assert!(
            msg.contains("tr-ab34"),
            "expected message to list candidate 'tr-ab34', got: {msg}"
        );
    }

    #[test]
    fn display_tickets_not_found() {
        let err = Error::TicketsNotFound;
        let msg = err.to_string();
        assert!(
            msg.contains("no .tickets directory found"),
            "expected message to contain 'no .tickets directory found', got: {msg}"
        );
    }

    #[test]
    fn display_invalid_status_names_value_and_valid_options() {
        let err = Error::InvalidStatus {
            value: "pending".to_string(),
            suggestion: None,
        };
        let msg = err.to_string();
        assert!(
            msg.contains("pending"),
            "expected message to contain bad value 'pending', got: {msg}"
        );
        assert!(
            msg.contains("open"),
            "expected message to list valid option 'open', got: {msg}"
        );
        assert!(
            msg.contains("in_progress"),
            "expected message to list valid option 'in_progress', got: {msg}"
        );
        assert!(
            msg.contains("closed"),
            "expected message to list valid option 'closed', got: {msg}"
        );
    }

    #[test]
    fn display_invalid_status_with_suggestion() {
        let err = Error::InvalidStatus {
            value: "in_progres".to_string(),
            suggestion: Some("in_progress".to_string()),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("did you mean: in_progress?"),
            "expected 'did you mean: in_progress?' in message, got: {msg}"
        );
    }

    #[test]
    fn display_invalid_status_no_suggestion_omits_hint() {
        let err = Error::InvalidStatus {
            value: "xyz".to_string(),
            suggestion: None,
        };
        let msg = err.to_string();
        assert!(
            !msg.contains("did you mean"),
            "expected no 'did you mean' when suggestion is None, got: {msg}"
        );
    }

    #[test]
    fn display_invalid_type_names_value_and_valid_options() {
        let err = Error::InvalidType {
            value: "story".to_string(),
            suggestion: None,
        };
        let msg = err.to_string();
        assert!(
            msg.contains("story"),
            "expected message to contain bad value 'story', got: {msg}"
        );
        for opt in &["bug", "feature", "task", "epic", "chore"] {
            assert!(
                msg.contains(opt),
                "expected message to list valid option '{opt}', got: {msg}"
            );
        }
    }

    #[test]
    fn display_invalid_type_with_suggestion() {
        let err = Error::InvalidType {
            value: "feeture".to_string(),
            suggestion: Some("feature".to_string()),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("did you mean: feature?"),
            "expected 'did you mean: feature?' in message, got: {msg}"
        );
    }

    #[test]
    fn display_invalid_type_no_suggestion_omits_hint() {
        let err = Error::InvalidType {
            value: "xyz".to_string(),
            suggestion: None,
        };
        let msg = err.to_string();
        assert!(
            !msg.contains("did you mean"),
            "expected no 'did you mean' when suggestion is None, got: {msg}"
        );
    }

    #[test]
    fn display_invalid_priority_names_value_and_valid_options() {
        let err = Error::InvalidPriority {
            value: "urgent".to_string(),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("urgent"),
            "expected message to contain bad value 'urgent', got: {msg}"
        );
        for opt in &["0", "1", "2", "3", "4"] {
            assert!(
                msg.contains(opt),
                "expected message to list valid option '{opt}', got: {msg}"
            );
        }
    }

    #[test]
    fn from_io_error_produces_io_variant() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err: Error = io_err.into();
        assert!(
            matches!(err, Error::Io(_)),
            "expected Io variant, got: {err:?}"
        );
        assert!(
            !err.to_string().is_empty(),
            "expected non-empty Display output for Io variant"
        );
    }

    #[test]
    fn from_yaml_error_produces_yaml_variant() {
        // Trigger a real serde_yaml parse error.
        let yaml_err = serde_yaml::from_str::<serde_yaml::Value>(
            r#":
  bad: [yaml"#,
        )
        .unwrap_err();
        let err: Error = yaml_err.into();
        assert!(
            matches!(err, Error::Yaml(_)),
            "expected Yaml variant, got: {err:?}"
        );
        assert!(
            !err.to_string().is_empty(),
            "expected non-empty Display output for Yaml variant"
        );
    }

    #[test]
    fn std_error_source_returns_some_for_io_variant() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err = Error::Io(io_err);
        assert!(
            err.source().is_some(),
            "expected source() to return Some for Io variant"
        );
    }

    #[test]
    fn std_error_source_returns_none_for_non_wrapping_variants() {
        let err = Error::TicketsNotFound;
        assert!(
            err.source().is_none(),
            "expected source() to return None for TicketsNotFound"
        );
    }
}
