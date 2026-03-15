# Contributing

All humans and agents should follow these instructions.

- All non-trivial work must have a corresponding ticket, dog-fooded by this tool.

## Git Commits 

- Commit messages include both a subject and a body.
- The Commit subject is short, imperative and explains _what_ the change is for. They should always begin with one of
  only these five prefixes:
  - Add
  - Update
  - Fix
  - Remove
  - Refactor
- Each Git commit almost always includes a body, which explains _why_ the commit was made. It needs to explain the following:
  - Why the change is necessary.
  - How the change was implemented.
- Commit bodies are written as paragraphs.
  - Each paragraph is properly capitalized and reads like a page out of a book. 
  - Each paragraph is devoted to a single idea and uses proper punctuation.
- If work corresponds to a ticket, include the ticket id and title in the body.

## Changelog

Follow [Common Changelog](https://common-changelog.org/) format. Key rules:

- **File**: `CHANGELOG.md`, starts with `# Changelog`
- **Release heading**: `## [VERSION] - YYYY-MM-DD` (no "v" prefix, version links to GitHub release)
- **Categories** (third-level headings, in order): `Changed`, `Added`, `Removed`, `Fixed`
- **Each entry**: `- <imperative description> ([#ref](url)) (Author)` — one line, self-describing
- **Breaking changes**: prefix with `**Breaking:**`, list first within their category
- **References are required**: link to ticket id or commit for every change
- **Exclude noise**: dotfile changes, dev-only dependency bumps, minor formatting tweaks
- **Merge related changes**: multiple commits for the same thing become one entry
- **Skip no-ops**: a commit and its revert cancel out — omit both

Example:

```md
## [2.1.0] - 2026-03-15

### Changed

- **Breaking:** rename `connect()` to `open()` ([#55](https://github.com/owner/name/pull/55))

### Added

- Add retry logic for transient failures ([#58](https://github.com/owner/name/pull/58)) (Alice)

### Fixed

- Fix race condition in worker pool ([`a1b2c3d`](https://github.com/owner/name/commit/a1b2c3d))

[2.1.0]: https://github.com/owner/name/releases/tag/v2.1.0
```

