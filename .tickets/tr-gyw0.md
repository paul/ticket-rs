---
id: tr-gyw0
status: open
deps: [tr-kspr]
links: []
created: 2026-03-08T06:32:00Z
type: task
priority: 2
assignee: Paul Sadauskas
parent: tr-r6um
tags: [phase-4, command]
---
# Implement update command

Create src/commands/update.rs. Implement update <id> [options] to modify any ticket field from the CLI. Options: --title (update first # heading), -d/--description (replace text between title and first ## heading), --design (replace/insert ## Design section), --acceptance (replace/insert ## Acceptance Criteria section), -p/--priority, -t/--type, -a/--assignee, --external-ref, --parent (validate exists). Tag management: --tags (replace all), --add-tags (merge deduped), --remove-tags (remove specific, delete field if empty). --tags is mutually exclusive with --add-tags/--remove-tags. Section insertion order: ## Design < ## Acceptance Criteria < ## Notes. Print ticket ID on success.

## Testing

Write unit tests in a `#[cfg(test)]` module in `src/commands/update.rs`. Factor out the string-manipulation logic into pure functions (e.g., `update_title`, `replace_section`, `update_tags`) so they can be tested without filesystem access.

- **`--title`**: update the `# Heading`; assert the new title appears and the old one is gone.
- **`-d` description**: replace the description text between the title and first `##`; assert new text is present and old text is absent.
- **`--design` — replace existing**: overwrite an existing `## Design` section; assert new content.
- **`--design` — insert when absent**: assert a `## Design` section is created when not previously present.
- **`--acceptance` — replace/insert**: same as design, for `## Acceptance Criteria`.
- **Section insertion order**: insert both `## Design` and `## Acceptance Criteria` in a file that has only `## Notes`; assert `## Design` comes first, then `## Acceptance Criteria`, then `## Notes`.
- **`-p` priority**: update priority; assert new value in frontmatter.
- **`-t` type**: update type; assert new value in frontmatter.
- **`-a` assignee**: update assignee; assert new value in frontmatter.
- **`--external-ref`**: update external-ref; assert new value in frontmatter.
- **`--parent` validation**: supply a non-existent parent ID; assert an error is returned.
- **`--tags` replaces all**: existing tags `[a, b]`, update with `--tags c,d`; assert tags become `[c, d]`.
- **`--add-tags` merges deduped**: existing tags `[a, b]`, add `--add-tags b,c`; assert tags become `[a, b, c]`.
- **`--remove-tags` removes specific**: existing tags `[a, b, c]`, remove `--remove-tags b`; assert tags become `[a, c]`.
- **`--remove-tags` deletes field when empty**: remove the only tag; assert the `tags` field is absent from frontmatter.
- **`--tags` mutual exclusivity with `--add-tags`**: assert an error when both are supplied.
- **`--tags` mutual exclusivity with `--remove-tags`**: assert an error when both are supplied.
- **Unmodified fields preserved**: update only the title; assert all frontmatter fields and sections not targeted by the update are byte-identical to the original.
- **Output**: assert stdout contains the ticket ID on success.

## BDD Integration Tests

There is no dedicated feature file for `update`. Validate it indirectly: the bash reference implementation handles `update` as an internal script helper that the other feature files exercise through `ticket show` output and field assertions. Once implemented, check for regressions by running the full suite:

```bash
TICKET_SCRIPT=./target/debug/ticket behave features/
```

Pay particular attention to `ticket_show.feature` and `ticket_listing.feature`, which rely on correct field values after mutations.
