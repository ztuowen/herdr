# Herdr Kanban Automation Scripts

These scripts are plugin-side orchestration helpers. They do not change Herdr core logic. They automate through the existing Herdr CLI/socket surface:

- `herdr kanban list|update`
- `herdr workspace create|close`
- `herdr pane read|close`
- `herdr pane run|read|close`
- `herdr agent wait|send`

Scripts live in `bin/` and are intentionally one-shot except for `herdr-kanban-run`.

## Scripts

```bash
bin/herdr-kanban-card-check <uuid>
bin/herdr-kanban-install
bin/herdr-kanban-sweep
bin/herdr-kanban-review
bin/herdr-kanban-dispatch
bin/herdr-kanban-run
```

`herdr-kanban-run` composes the one-shot scripts:

```bash
while true; do
  herdr-kanban-sweep
  herdr-kanban-review
  herdr-kanban-dispatch
  sleep "$HERDR_KANBAN_INTERVAL"
done
```

## Safety Defaults

The scripts default to dry-run mode:

```bash
HERDR_KANBAN_DRY_RUN=1
```

Set `HERDR_KANBAN_DRY_RUN=0` to let the runner create workspaces, start Codex panes, update assignment metadata, and close disposable panes.

Install command symlinks into `${HERDR_KANBAN_INSTALL_DIR:-$HOME/.local/bin}`:

```bash
bin/herdr-kanban-install
```

The installer refuses to replace non-symlink files unless `HERDR_KANBAN_INSTALL_FORCE=1` is set.

Spawned Codex panes run in YOLO mode:

```bash
codex --dangerously-bypass-approvals-and-sandbox --cd <worktree> "<role prompt>"
```

YOLO applies only to spawned Codex panes. The runner itself should still avoid irreversible repository or external actions unless a future script explicitly opts into them.

## Configuration

```bash
HERDR_BIN=herdr
CODEX_BIN=codex
HERDR_KANBAN_DRY_RUN=1
HERDR_KANBAN_WORKTREE="$(pwd)"
HERDR_KANBAN_MAX_WORKERS=1
HERDR_KANBAN_MAX_REVIEWERS=1
HERDR_KANBAN_INTERVAL=10
HERDR_KANBAN_AGENT_PREFIX=kanban
HERDR_KANBAN_WORKSPACE_PREFIX=kanban
```

## Dispatch Flow

`herdr-kanban-dispatch`:

1. Lists `todo` cards.
2. Skips `clarify:` cards, invalid cards, and cards with an existing `assigned_pane`.
3. Creates a disposable workspace for each selected card.
4. Runs Codex in that workspace's root pane with `herdr pane run`.
5. Records `owner_role`, `assigned_pane`, `assigned_workspace`, and `last_actor` in the card description frontmatter.

The worker prompt instructs the spawned agent to attach its own pane:

```bash
herdr kanban attach <uuid>
herdr kanban update <uuid> --status ongoing
```

## Review Flow

`herdr-kanban-review`:

1. Lists `reviewing` cards.
2. Skips cards already assigned to a reviewer.
3. Requires a `## Validation Evidence` section.
4. Creates a disposable reviewer workspace.
5. Runs Codex in that workspace's root pane in YOLO mode with the reviewer skill prompt.

## Sweep Flow

`herdr-kanban-sweep`:

1. Clears stale assignment metadata when a card points at a missing pane.
2. Warns on `reviewing` cards without validation evidence.
3. Warns on `blocked` cards with an empty `blocked_reason`.
4. Closes disposable `kanban-*` panes that are `idle` or `done`.
5. Closes empty disposable `kanban *` workspaces.

## Card Readiness

`herdr-kanban-card-check <uuid>` requires:

- `## Objective`
- `## Context`
- `## Scope`
- `## Out of Scope`
- `## Acceptance`
- `## Validation`
- `## Handoff Rules`

The checker rejects `clarify:` cards and cards without a description file.
