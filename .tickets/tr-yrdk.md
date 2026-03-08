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

