---
id: tr-siyb
status: closed
deps: [tr-6u6p]
links: []
created: 2026-03-08T06:30:08Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-3kr6
tags: [phase-1, cli]
---
# Implement clap CLI skeleton

Create src/cli.rs with clap derive-based command definitions. Define the top-level Cli struct and Commands enum with all subcommands stubbed out. Start with functional definitions for Phase 1 commands (create, show, start, close, reopen, status). Other phases' commands can be defined as stubs that print 'not yet implemented'. Include global --color=auto|always|never flag. The binary should dispatch to command handler functions in src/commands/.

After parsing args, apply the --color flag globally using `console::set_colors_enabled()`: `always` forces colors on, `never` forces them off, `auto` (default) defers to console's built-in TTY + `NO_COLOR`/`CLICOLOR` detection. All output code should use `console::style()` for coloring rather than writing ANSI escapes manually — console's global state will then gate color output correctly throughout the binary.

## BDD Integration Tests

The CLI skeleton is a prerequisite for all BDD tests. Once stub commands are wired (even those that print "not yet implemented"), smoke-test the harness with:

```bash
cargo build
TICKET_SCRIPT=./target/debug/ticket behave features/ticket_creation.feature
```

As each Phase 1 command is implemented in subsequent tickets, run its corresponding feature file. The full suite is run once all commands are complete:

```bash
TICKET_SCRIPT=./target/debug/ticket behave features/
```

The bash implementation serves as the reference baseline — all 123 scenarios in all 12 feature files pass against it.

