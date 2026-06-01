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
bin/herdr-kanban-human-audit
bin/herdr-kanban-dispatch
bin/herdr-kanban-run
```

`herdr-kanban-run` composes the one-shot scripts:

```bash
while true; do
  herdr-kanban-sweep
  herdr-kanban-review
  herdr-kanban-human-audit
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
It also links the internal `herdr-kanban-lib` helper beside the runnable commands so installed symlinks can source their shared shell functions.

Spawned Codex panes run in YOLO mode. They default to interactive Codex because the sweeper closes disposable panes by watching for Herdr-visible `idle` or `done` agent states. `codex exec` exits the process and can leave the pane in `unknown`, so use `exec` only when another cleanup path owns those residual panes.

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
HERDR_KANBAN_CODEX_MODE=interactive
HERDR_KANBAN_WORKER_CODEX_MODE=interactive
HERDR_KANBAN_REVIEWER_CODEX_MODE=interactive
HERDR_KANBAN_HUMAN_AUDIT_CODEX_MODE=interactive
```

## Dispatch Flow

`herdr-kanban-dispatch`:

1. Lists `todo` cards.
2. Skips `clarify:` cards, invalid cards, and cards with an existing `assigned_pane`.
3. Creates a disposable workspace for each selected card.
4. Immediately attaches the card to that root pane with `HERDR_PANE_ID=<pane> herdr kanban attach <uuid>`.
5. Records `owner_role`, `assigned_pane`, `assigned_workspace`, and `last_actor` in the card description frontmatter.
6. Runs Codex in that workspace's root pane with `herdr pane run`, using `HERDR_KANBAN_WORKER_CODEX_MODE`.

The dispatcher attaches before starting Codex so the sweeper can distinguish a freshly launched worker from an unowned disposable pane. The worker prompt starts from the already attached pane and moves the card to ongoing:

```bash
herdr kanban update <uuid> --status ongoing
```

The prompt also includes the card description file path so the worker can update the handoff document directly.
Worker handoff to review requires a committed and pushed task branch. The worker records `branch_name` and `commit_sha`, clears worker assignment metadata, then moves the card to `reviewing`.

## Review Flow

`herdr-kanban-review`:

1. Lists `reviewing` cards.
2. Skips cards already assigned to a reviewer.
3. Requires a `## Validation Evidence` section.
4. Creates a disposable reviewer workspace.
5. Immediately attaches the card to that root pane with `HERDR_PANE_ID=<pane> herdr kanban attach <uuid>`.
6. Runs Codex in that workspace's root pane in YOLO mode with the reviewer skill prompt, using `HERDR_KANBAN_REVIEWER_CODEX_MODE`.

The reviewer prompt includes the card description file path so review notes and routing decisions can be recorded directly in the handoff document.
Reviewer launch attaches before starting Codex so Herdr-native card tracking points at the active review pane from creation.
Reviewer acceptance owns the merge gate: the reviewer inspects the pushed branch, merges or fast-forwards according to repo rules, pushes the integration branch when allowed, then moves the card to `done`.
Rejected review writes findings, clears assignment metadata, and moves the card back to `todo` so the dispatcher can assign a fresh worker.
When review needs human input, reviewers write the next ask into `## Human Review Request`, set `review_state` to `human-review-required` or `triage-question-required`, set `blocked_reason`, and move the card to `blocked`; triage or the coordinator surfaces the ask.

## Human Audit Flow

`herdr-kanban-human-audit`:

1. Lists `blocked` cards.
2. Selects cards with `review_state: human-review-required`, `triage-question-required`, or `design-decision-required`.
3. Skips cards that already have `owner_role`, `assigned_pane`, or `assigned_workspace`; active assignment is the duplicate-spawn guard.
4. Creates a disposable human-audit workspace.
5. Immediately attaches the card to that root pane with `HERDR_PANE_ID=<pane> herdr kanban attach <uuid>`.
6. Records `owner_role: human-audit`, `assigned_pane`, `assigned_workspace`, and `last_actor`.
7. Runs Codex with the human-audit prompt, using `HERDR_KANBAN_HUMAN_AUDIT_CODEX_MODE`.

The human-audit skill inspects all available card, review, branch, diff, docs, and artifact context before asking the human anything. It records a `## Human Audit Checklist` that covers every original blocked reason, asks one focused question at a time with a recommended answer, and cannot release the parent card until the original checklist is addressed. It may create follow-up `clarify:` blocker cards on the spot; those children are non-parent-blocking unless explicitly marked otherwise.

When the parent is resolved, the audit agent uses the brushup handoff skill to write the decision, clear assignment and blocker metadata, move the parent to `reviewing` or `todo` as appropriate, detach, and exit. If unresolved, the card stays `blocked` and assigned to the open audit pane.

## Sweep Flow

`herdr-kanban-sweep`:

1. Clears stale assignment metadata when a card points at a missing pane.
2. Warns on `reviewing` cards without validation evidence.
3. Warns on `blocked` cards with an empty `blocked_reason`.
4. Closes assigned panes that are `idle` or `done`.
5. Closes unassigned `idle` or `done` panes in disposable `kanban worker *`, `kanban reviewer *`, and `kanban human-audit *` workspaces.
6. Keeps `todo`, `ongoing`, and blocked `human-audit` card panes protected from cleanup while launch, active work, or audit is in progress.
7. Closes empty disposable `kanban *` workspaces.

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
