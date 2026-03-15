---
id: tr-d9da
status: closed
deps: []
links: []
created: 2026-03-15T22:21:18Z
type: chore
priority: 2
assignee: Paul Sadauskas
---
# Extract filter helpers into Ticket

The assignee and tag filter logic is duplicated identically four times in list.rs (in format_list, format_ready, format_blocked, and format_closed_from_paths). This makes the code harder to maintain and means any behavior change must be made in four places.

Add two methods to Ticket in ticket.rs:
- has_tag(&self, tag: &str) -> bool: checks if the ticket has the given tag
- matches_filters(&self, assignee: Option<&str>, tag: Option<&str>) -> bool: combines the assignee and tag checks

Update all four filter sites in list.rs to call t.matches_filters(assignee, tag) instead of the duplicated 10-line blocks. No behavior change.
