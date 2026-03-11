// Ticket ID generation: prefix from directory name + 4-char random hex suffix.

/// Derive a 2-4 character prefix from a directory name.
///
/// Multi-segment names (split on `-` or `_`) use the first letter of each
/// segment, truncated to 4 characters. Single-segment names use the full name
/// if it is 3 characters or shorter, otherwise the first 3 characters.
pub fn derive_prefix(dir_name: &str) -> String {
    let segments: Vec<&str> = dir_name.split(['-', '_']).collect();
    if segments.len() > 1 {
        segments
            .iter()
            .filter_map(|s| s.chars().next())
            .take(4)
            .collect()
    } else if dir_name.len() <= 3 {
        dir_name.to_string()
    } else {
        dir_name[..3].to_string()
    }
}

/// Generate a ticket ID in the form `PREFIX-SUFFIX`.
///
/// The prefix is derived from `dir_name` via [`derive_prefix`]. The suffix is
/// 4 random lowercase hexadecimal characters.
pub fn generate_id(dir_name: &str) -> String {
    let prefix = derive_prefix(dir_name);
    generate_id_with_prefix(&prefix)
}

/// Generate a ticket ID in the form `PREFIX-SUFFIX` using an explicit prefix.
///
/// Unlike [`generate_id`], this skips prefix derivation and uses the supplied
/// string directly. The suffix is 4 random lowercase hexadecimal characters.
pub fn generate_id_with_prefix(prefix: &str) -> String {
    let suffix: u16 = rand::random();
    format!("{}-{:04x}", prefix, suffix)
}

/// Normalise a user-supplied prefix by stripping a trailing `-` if present.
///
/// This allows users to write either `tk` or `tk-` in their config and get
/// identical behaviour, since the `-` separator is always added by
/// [`generate_id_with_prefix`].
pub fn normalise_prefix(prefix: &str) -> &str {
    prefix.strip_suffix('-').unwrap_or(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_segment_two() {
        assert_eq!(derive_prefix("my-project"), "mp");
    }

    #[test]
    fn multi_segment_three() {
        assert_eq!(derive_prefix("ticket-cli-rs"), "tcr");
    }

    #[test]
    fn multi_segment_four() {
        assert_eq!(derive_prefix("my-big-rust-app"), "mbra");
    }

    #[test]
    fn single_segment_short() {
        assert_eq!(derive_prefix("tk"), "tk");
    }

    #[test]
    fn single_segment_long() {
        assert_eq!(derive_prefix("platform"), "pla");
    }

    #[test]
    fn single_segment_exactly_three() {
        assert_eq!(derive_prefix("foo"), "foo");
    }

    #[test]
    fn underscore_delimiter() {
        assert_eq!(derive_prefix("my_project"), "mp");
    }

    #[test]
    fn mixed_delimiters() {
        assert_eq!(derive_prefix("my_big-app"), "mba");
    }

    #[test]
    fn generate_id_format() {
        let id = generate_id("my-project");
        let re = regex_lite(&id);
        assert!(re, "id '{id}' did not match ^[a-z]{{2,4}}-[0-9a-f]{{4}}$");
    }

    #[test]
    fn generate_id_suffix_is_hex() {
        let id = generate_id("my-project");
        let suffix = id.split('-').next_back().unwrap();
        assert!(
            suffix.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')),
            "suffix '{suffix}' contains non-lowercase-hex characters"
        );
    }

    #[test]
    fn generate_id_suffix_length() {
        let id = generate_id("my-project");
        let suffix = id.split('-').next_back().unwrap();
        assert_eq!(suffix.len(), 4, "suffix '{suffix}' is not 4 characters");
    }

    /// Simple regex check without pulling in a regex crate.
    fn regex_lite(id: &str) -> bool {
        let Some((prefix, suffix)) = id.split_once('-') else {
            return false;
        };
        let prefix_ok =
            (2..=4).contains(&prefix.len()) && prefix.chars().all(|c| c.is_ascii_lowercase());
        let suffix_ok =
            suffix.len() == 4 && suffix.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'));
        prefix_ok && suffix_ok
    }
}
