---
id: tr-yrdk
status: open
deps: [tr-fz7v]
links: []
created: 2026-03-08T06:32:43Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-09dq
tags: [phase-6, polish]
---
# Implement syntax highlighting for show output

Create src/highlight.rs. Use syntect crate (default-fancy feature, pure Rust) to syntax-highlight ticket output. SyntaxSet::load_defaults_newlines() for bundled YAML and Markdown grammars. Split show output at frontmatter delimiters: highlight --- to --- block as YAML, remainder as Markdown. Use as_24_bit_terminal_escaped() for ANSI output. Integrate into show command.

Respect color settings by checking `console::colors_enabled()` — this single call already accounts for TTY detection, `NO_COLOR`, `CLICOLOR`, and the `--color` flag applied at startup via `console::set_colors_enabled()`. Do not re-implement that logic here; just gate the highlighter on `colors_enabled()`.

## BDD Integration Tests

Syntax highlighting is not directly tested by the BDD suite — tests run non-TTY so colors are off and ANSI codes are absent. What matters for BDD purposes is that `show` output remains parseable when highlighting is disabled. Run after integration to confirm no regressions:

```bash
TICKET_SCRIPT=./target/debug/ticket behave features/ticket_show.feature
```

Validate highlighting visually by running `./target/debug/ticket show <id>` in a TTY with a real ticket.
