// Shared ticket-line formatting helpers.
//
// `build_line` is the canonical way to render a single ticket as a display
// line.  It is used by the `tree`, `ls`/`list`, `ready`, `blocked`, `closed`,
// and `dep tree` commands so that all output shares the same look:
//
//   {prefix}{connector}{id} {priority} {status} {title}[ [{deps}]][ {#tags}]
//
// For flat-list output (ls, ready, blocked, closed) pass `prefix = ""` and
// `connector = ""`; for tree output pass the accumulated indentation and the
// appropriate box-drawing connector.
//
// Color is applied via the `console` crate and respects `NO_COLOR` /
// `CLICOLOR` env vars and the global `--color` flag automatically.
//
// Terminal-width truncation is applied when `term_width` is `Some`.  When
// stdout is piped (not a TTY) callers should pass `None` so that scripted
// consumers receive the full content.

use std::collections::HashMap;

use console::style;

use crate::ticket::{Status, Ticket};

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

/// Return the colored status label (no brackets).
pub fn status_label(status: &Status) -> String {
    let label = status.to_string();
    match status {
        Status::InProgress => format!("{}", style(label).cyan()),
        Status::Open => format!("{}", style(label).blue()),
        Status::Closed => format!("{}", style(label).dim()),
    }
}

/// Return the colored priority label (e.g. `P2`).
///
/// P0 = red, P1 = yellow (orange on most terminals), P2 = magenta (purple),
/// P3 = dim, P4 = black.
pub fn priority_label(priority: u8) -> String {
    let label = format!("P{priority}");
    match priority {
        0 => format!("{}", style(label).red()),
        1 => format!("{}", style(label).yellow()),
        2 => format!("{}", style(label).magenta()),
        3 => format!("{}", style(label).dim()),
        _ => format!("{}", style(label).black()),
    }
}

/// Return a dep ID styled by the dep ticket's status.  Falls back to dim if
/// the dep is not in the provided ticket map.
pub fn dep_id_label(dep_id: &str, tickets: &HashMap<String, Ticket>) -> String {
    match tickets.get(dep_id).map(|t| &t.status) {
        Some(Status::InProgress) => format!("{}", style(dep_id).cyan()),
        Some(Status::Open) => format!("{}", style(dep_id).blue()),
        Some(Status::Closed) | None => format!("{}", style(dep_id).dim()),
    }
}

// ---------------------------------------------------------------------------
// Width measurement
// ---------------------------------------------------------------------------

/// Strip ANSI escape codes from `s` and return its display width in
/// characters.  Used for terminal-width budgeting in `build_line`.
pub fn display_width(s: &str) -> usize {
    let mut width = 0usize;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for ch in chars.by_ref() {
                    if ch.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            width += 1;
        }
    }
    width
}

// ---------------------------------------------------------------------------
// Line assembly
// ---------------------------------------------------------------------------

/// Assemble and return a single formatted ticket line, applying
/// terminal-width truncation when `term_width` is `Some`.
///
/// The line has the form:
///
/// ```text
/// {prefix}{connector}{id} {priority} {status} {title}[{deps_suffix}][{tags_suffix}]
/// ```
///
/// For flat output pass `prefix = ""` and `connector = ""`.
///
/// When the line exceeds `term_width` columns the following steps are tried
/// in order until it fits:
///   1. Drop the tags suffix.
///   2. Truncate the title with `"…"` (keeping deps visible).
///   3. Drop the deps suffix (last resort).
///
/// If `term_width` is `None` no truncation is applied (pipe / redirect mode).
#[allow(clippy::too_many_arguments)]
pub fn build_line(
    prefix: &str,
    connector: &str,
    id_part: &str,
    priority_part: &str,
    status_part: &str,
    title: &str,
    deps_suffix: &str,
    tags_suffix: &str,
    term_width: Option<usize>,
) -> String {
    // Assemble with all optional parts.
    let full = format!(
        "{prefix}{connector}{id_part} {priority_part} {status_part} {title}{deps_suffix}{tags_suffix}"
    );

    let Some(width) = term_width else {
        return full;
    };

    if display_width(&full) <= width {
        return full;
    }

    // Step 1: try without tags.
    let no_tags =
        format!("{prefix}{connector}{id_part} {priority_part} {status_part} {title}{deps_suffix}");
    if display_width(&no_tags) <= width {
        return no_tags;
    }

    // Step 2: truncate the title (keeping deps).
    let overhead_no_deps = format!("{prefix}{connector}{id_part} {priority_part} {status_part} ");
    let overhead_nd = display_width(&overhead_no_deps);
    let deps_width = display_width(deps_suffix);
    if overhead_nd + 1 + deps_width <= width {
        let title_chars_budget = width - overhead_nd - deps_width;
        let truncated_title: String = if title.chars().count() <= title_chars_budget {
            title.to_string()
        } else if title_chars_budget <= 1 {
            "…".to_string()
        } else {
            let mut s: String = title.chars().take(title_chars_budget - 1).collect();
            s.push('…');
            s
        };
        let candidate = format!(
            "{prefix}{connector}{id_part} {priority_part} {status_part} {truncated_title}{deps_suffix}"
        );
        if display_width(&candidate) <= width {
            return candidate;
        }
    }

    // Step 3: drop deps entirely and truncate title against the narrower budget.
    let overhead_bare = overhead_nd;
    if overhead_bare < width {
        let title_chars_budget = width - overhead_bare;
        let truncated_title: String = if title.chars().count() <= title_chars_budget {
            title.to_string()
        } else if title_chars_budget <= 1 {
            "…".to_string()
        } else {
            let mut s: String = title.chars().take(title_chars_budget - 1).collect();
            s.push('…');
            s
        };
        return format!(
            "{prefix}{connector}{id_part} {priority_part} {status_part} {truncated_title}"
        );
    }

    // Absolute minimum: fixed parts only, plus deps when present.
    if deps_suffix.is_empty() {
        format!("{prefix}{connector}{id_part} {priority_part} {status_part}")
    } else {
        format!("{prefix}{connector}{id_part} {priority_part} {status_part} {deps_suffix}")
    }
}
