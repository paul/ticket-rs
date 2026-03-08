---
id: tr-hs04
status: open
deps: [tr-siyb]
links: []
created: 2026-03-08T06:32:32Z
type: task
priority: 1
assignee: Paul Sadauskas
parent: tr-09dq
tags: [phase-6, plugins]
---
# Implement external plugin discovery and dispatch

Create src/plugin.rs. When a command is not a built-in, search PATH for executables named ticket-<cmd> then tk-<cmd>. If found, exec the plugin with remaining args, passing TICKETS_DIR and TK_SCRIPT (path to own binary) as env vars. Implement discover_plugins() -> Vec<PluginInfo> that scans PATH for all ticket-* and tk-* executables, extracts descriptions from '# tk-plugin:' comment in first 10 lines (for scripts) or --tk-describe flag output (for binaries). Used by help command to list available plugins.

## Testing

Write unit tests in a `#[cfg(test)]` module in `src/plugin.rs`. Use `tempfile::tempdir()` for tests that create fake plugin scripts.

- **`tk-` prefix discovered**: create an executable `tk-hello` in a temp dir on `PATH`; assert `discover_plugins()` returns a `PluginInfo` with name `"hello"`.
- **`ticket-` prefix discovered**: create an executable `ticket-greet` in a temp dir on `PATH`; assert it is discovered with name `"greet"`.
- **`tk-` prefix takes precedence**: create both `tk-test` and `ticket-test` in the same dir; assert only the `tk-` version is returned (no duplicate).
- **Description extracted from comment**: write a script with `# tk-plugin: My custom plugin` in the first 10 lines; assert `PluginInfo.description` is `"My custom plugin"`.
- **No description**: a script with no `# tk-plugin:` comment; assert `PluginInfo.description` is `None` (or displayed as `"(no description)"`).
- **Non-plugin executables ignored**: create an executable `tk` and `ticket` (no hyphen-suffix); assert they are not returned by `discover_plugins()`.
- **`TICKETS_DIR` passed to plugin**: verify the env-building logic sets `TICKETS_DIR` to the resolved tickets directory.
- **`TK_SCRIPT` passed to plugin**: verify the env-building logic sets `TK_SCRIPT` to the path of the current binary.
- **Command not in PATH returns `None`**: call plugin lookup for a command that has no `tk-` or `ticket-` executable in `PATH`; assert `None` is returned so the built-in fallback can handle it.
