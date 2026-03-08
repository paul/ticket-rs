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

