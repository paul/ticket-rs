// Fuzzy suggestion utilities for did-you-mean hints.
//
// Used when a user provides an invalid ticket ID, status, or ticket type to
// surface the closest known match rather than showing a bare error.

use strsim::jaro_winkler;

use crate::ticket::Ticket;

/// Minimum Jaro-Winkler similarity score required for a suggestion to be shown.
const TICKET_THRESHOLD: f64 = 0.7;
const KEYWORD_THRESHOLD: f64 = 0.7;

/// Find tickets whose IDs are similar to `input`.
///
/// Computes Jaro-Winkler similarity between `input` and each ticket's ID.
/// Returns up to `max` tickets whose similarity exceeds [`TICKET_THRESHOLD`],
/// ordered by similarity descending (best match first). The returned tickets
/// are clones of the original so the caller can render them freely.
pub fn suggest_tickets(input: &str, tickets: &[Ticket], max: usize) -> Vec<Ticket> {
    let mut scored: Vec<(f64, &Ticket)> = tickets
        .iter()
        .filter_map(|t| {
            let score = jaro_winkler(input, &t.id);
            if score >= TICKET_THRESHOLD {
                Some((score, t))
            } else {
                None
            }
        })
        .collect();

    // Sort by score descending, then by ID ascending as a tiebreaker for
    // deterministic output.
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.id.cmp(&b.1.id))
    });

    scored
        .into_iter()
        .take(max)
        .map(|(_, t)| t.clone())
        .collect()
}

/// Find the closest match for `input` among a set of known keyword strings.
///
/// Returns `Some(best_match)` if the top candidate exceeds [`KEYWORD_THRESHOLD`],
/// or `None` if no candidate is similar enough.
pub fn suggest_keyword(input: &str, candidates: &[&str]) -> Option<String> {
    candidates
        .iter()
        .map(|&c| (jaro_winkler(input, c), c))
        .filter(|(score, _)| *score >= KEYWORD_THRESHOLD)
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, c)| c.to_string())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    use crate::ticket::{Status, TicketType};

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
            title: format!("Ticket {id}"),
            body: format!("# Ticket {id}\n"),
        }
    }

    // -----------------------------------------------------------------------
    // suggest_tickets — near-miss returns suggestion
    // -----------------------------------------------------------------------

    #[test]
    fn suggest_tickets_near_miss_returns_match() {
        let tickets = vec![make_ticket("tr-abcd"), make_ticket("tr-efgh")];
        let suggestions = suggest_tickets("tr-abce", &tickets, 3);
        assert!(
            !suggestions.is_empty(),
            "expected at least one suggestion for 'tr-abce' near 'tr-abcd'"
        );
        assert_eq!(
            suggestions[0].id, "tr-abcd",
            "expected 'tr-abcd' as first suggestion"
        );
    }

    // -----------------------------------------------------------------------
    // suggest_tickets — no similar match returns empty
    // -----------------------------------------------------------------------

    #[test]
    fn suggest_tickets_no_match_returns_empty() {
        let tickets = vec![make_ticket("tr-abcd"), make_ticket("tr-efgh")];
        // Completely unrelated string should produce no suggestions.
        let suggestions = suggest_tickets("zzz-9999", &tickets, 3);
        assert!(
            suggestions.is_empty(),
            "expected no suggestions for 'zzz-9999', got: {:?}",
            suggestions.iter().map(|t| &t.id).collect::<Vec<_>>()
        );
    }

    // -----------------------------------------------------------------------
    // suggest_tickets — respects max parameter
    // -----------------------------------------------------------------------

    #[test]
    fn suggest_tickets_respects_max() {
        let tickets = vec![
            make_ticket("tr-aaaa"),
            make_ticket("tr-aaab"),
            make_ticket("tr-aaac"),
            make_ticket("tr-aaad"),
        ];
        let suggestions = suggest_tickets("tr-aaaa", &tickets, 2);
        assert!(
            suggestions.len() <= 2,
            "expected at most 2 suggestions, got {}",
            suggestions.len()
        );
    }

    // -----------------------------------------------------------------------
    // suggest_tickets — best match comes first
    // -----------------------------------------------------------------------

    #[test]
    fn suggest_tickets_ordered_by_similarity() {
        let tickets = vec![
            make_ticket("tr-abcd"),
            make_ticket("tr-abce"), // one char off — closer
        ];
        // "tr-abce" should score higher than "tr-abcd" for input "tr-abce"
        let suggestions = suggest_tickets("tr-abce", &tickets, 3);
        assert!(
            !suggestions.is_empty(),
            "expected suggestions for 'tr-abce'"
        );
        assert_eq!(
            suggestions[0].id,
            "tr-abce",
            "expected exact match first: {:?}",
            suggestions.iter().map(|t| &t.id).collect::<Vec<_>>()
        );
    }

    // -----------------------------------------------------------------------
    // suggest_tickets — empty ticket list returns empty
    // -----------------------------------------------------------------------

    #[test]
    fn suggest_tickets_empty_list_returns_empty() {
        let suggestions = suggest_tickets("tr-abcd", &[], 3);
        assert!(
            suggestions.is_empty(),
            "expected empty result for empty list"
        );
    }

    // -----------------------------------------------------------------------
    // suggest_keyword — near-miss returns best match
    // -----------------------------------------------------------------------

    #[test]
    fn suggest_keyword_near_miss_returns_match() {
        let candidates = &["open", "in_progress", "closed"];
        let result = suggest_keyword("in_progres", candidates);
        assert_eq!(
            result.as_deref(),
            Some("in_progress"),
            "expected 'in_progress' for 'in_progres'"
        );
    }

    // -----------------------------------------------------------------------
    // suggest_keyword — no similar match returns None
    // -----------------------------------------------------------------------

    #[test]
    fn suggest_keyword_no_match_returns_none() {
        let candidates = &["open", "in_progress", "closed"];
        let result = suggest_keyword("xyz", candidates);
        assert!(result.is_none(), "expected None for 'xyz', got: {result:?}");
    }

    // -----------------------------------------------------------------------
    // suggest_keyword — exact match returns that value
    // -----------------------------------------------------------------------

    #[test]
    fn suggest_keyword_exact_match() {
        let candidates = &["bug", "feature", "task", "epic", "chore"];
        let result = suggest_keyword("feature", candidates);
        assert_eq!(
            result.as_deref(),
            Some("feature"),
            "expected 'feature' for exact input 'feature'"
        );
    }

    // -----------------------------------------------------------------------
    // suggest_keyword — typo in ticket type
    // -----------------------------------------------------------------------

    #[test]
    fn suggest_keyword_typo_in_type() {
        let candidates = &["bug", "feature", "task", "epic", "chore"];
        let result = suggest_keyword("feeture", candidates);
        assert_eq!(
            result.as_deref(),
            Some("feature"),
            "expected 'feature' for 'feeture'"
        );
    }

    // -----------------------------------------------------------------------
    // suggest_keyword — empty candidates returns None
    // -----------------------------------------------------------------------

    #[test]
    fn suggest_keyword_empty_candidates_returns_none() {
        let result = suggest_keyword("open", &[]);
        assert!(result.is_none(), "expected None for empty candidates list");
    }
}
