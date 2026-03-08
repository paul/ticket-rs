---
id: tr-23te
status: open
deps: [tr-aaqe]
links: []
created: 2026-03-08T06:31:03Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-xevn
tags: [phase-2, command]
---
# Implement dep cycle command

Add dep cycle subcommand. Run DFS-based cycle detection across all open/in_progress tickets. Only consider open and in_progress tickets (skip closed). Normalize and deduplicate detected cycles. Output each cycle showing the chain (a -> b -> c -> a) and listing each ticket with [status] and title. Exit 0 if no cycles found, exit 1 if cycles detected.

## Testing

Write unit tests in a `#[cfg(test)]` module in `src/commands/dep.rs` (or a dedicated cycle-detection module). Test the DFS algorithm with in-memory ticket graphs.

- **No cycles — exits clean**: a ticket graph with no cycles; assert the function returns an empty cycle list and exit code 0.
- **Simple two-node cycle**: A → B → A; assert one cycle is detected containing both A and B, and exit code is 1.
- **Three-node cycle**: A → B → C → A; assert one cycle containing all three is detected.
- **Multiple independent cycles**: A → B → A and C → D → C; assert both cycles are detected.
- **Cycle normalization**: a cycle can be detected starting at any node in the loop; assert the output represents each cycle only once regardless of traversal order.
- **Closed tickets skipped**: A → B → A but B is closed; assert no cycle is reported (closed tickets are excluded from detection).
- **in_progress tickets included**: A is `in_progress`, B is `open`, A → B → A; assert the cycle is still detected.
- **Non-cyclic dep chain**: A → B → C (linear); assert no cycle is detected.
- **Output format**: assert the cycle chain is printed as `"a -> b -> c -> a"` with each ticket on a subsequent line showing `[status]` and title.
