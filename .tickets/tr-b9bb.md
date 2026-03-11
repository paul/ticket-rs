---
id: tr-b9bb
status: closed
deps: []
links: []
created: 2026-03-11T16:48:49Z
type: feature
priority: 2
assignee: Paul Sadauskas
---
# Add show-config command

Add a show-config subcommand that prints the resolved value and source of every config key (ticket_prefix, ticket_dir). Always shows the effective value, even for defaults. Sources are: default, .tickets.toml (with path), or env var (with name). Requires adding source tracking (Source enum) to the Config struct, a new show_config command module, CLI variant, dispatch wiring, and BDD scenarios.
