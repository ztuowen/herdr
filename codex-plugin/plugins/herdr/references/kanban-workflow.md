# Herdr Kanban Workflow Contract

Herdr Kanban is the only handoff surface. Agents must not rely on chat history, private notes, or implicit context after triage. A downstream agent should be able to resume from:

- Kanban card title
- Kanban card description
- Kanban status
- attached pane
- Herdr-visible agent status

Ready cards must be tracer bullets: each card should produce an independently demonstrable end-to-end slice, not just a horizontal layer that waits on other cards. A card may build infrastructure, but it still needs an observable proof such as a diagnostics panel, CLI/API invocation, integration test, fixture-backed mock mode, or visible UI state.

Dependencies must be bounded. If a dependency is unavailable, the card should describe the fallback path that still lets a worker produce reviewable progress: mock data, fixture input, a standalone route, a diagnostic panel, a focused test, or a manual invocation.

## Commands

```bash
herdr kanban add "<title>" --description <path.md> --status todo
herdr kanban list
herdr kanban list --status todo
herdr kanban list --status reviewing
herdr kanban update <uuid> --title "<title>"
herdr kanban update <uuid> --description <path.md>
herdr kanban update <uuid> --status <todo|ongoing|blocked|reviewing|done>
herdr kanban attach <uuid>
herdr kanban detach <uuid>
```

For automation-launched worker and reviewer panes, the dispatcher runs `herdr kanban attach` with `HERDR_PANE_ID` set to the new pane before starting the agent. Manually claimed work should still run `herdr kanban attach` inside the pane that owns the work.

## Plugin Automation Scripts

The plugin includes complementary orchestration scripts under `bin/`. These scripts use Herdr's existing CLI/socket API and do not require Herdr core changes.

```bash
herdr-kanban-card-check <uuid>
herdr-kanban-install
herdr-kanban-sweep
herdr-kanban-review
herdr-kanban-human-audit
herdr-kanban-dispatch
herdr-kanban-run
```

They default to dry-run mode. Set `HERDR_KANBAN_DRY_RUN=0` to let them create Herdr workspaces, spawn Codex panes, write assignment metadata, and close disposable panes.
Dispatch, review, and human-audit launchers attach newly created panes before starting Codex so the sweeper can protect freshly launched work from early cleanup.

Spawned Codex panes use YOLO mode:

```bash
codex --dangerously-bypass-approvals-and-sandbox --cd <worktree> "<role prompt>"
```

See `kanban-automation-scripts.md` and `kanban-orchestration-flow.html` for the operational plan.

## Status Semantics

- `todo`: ready routine work, unless prefixed with `clarify:`.
- `ongoing`: claimed by an attached worker pane.
- `blocked`: waiting on human input, human review, or other manual intervention.
- `reviewing`: ready for or already owned by a review agent after implementation and validation evidence are recorded.
- `done`: accepted by reviewer or human.

Because the current Herdr Kanban status set has no dedicated `human-review` column, encode human presentation review as `blocked` plus metadata.

Blocked cards that require interactive human resolution should use one of:

- `review_state: human-review-required`
- `review_state: triage-question-required`
- `review_state: design-decision-required`

The human-audit launcher owns these cards when they have no active assignment.

## Title Prefixes

- `clarify:` means triage still owns clarification or research.

## Card Template

```markdown
---
workflow_state: ready
owner_role:
assigned_pane:
branch_name:
commit_sha:
review_state:
blocked_reason:
last_actor: triage
---

## Objective

## Context

## Scope

## Out of Scope

## Acceptance
- 

## Validation
- 

## Dependencies
- 

## Tracer Bullet Proof
- What end-to-end behavior this card must demonstrate:
- How to demonstrate it if related cards are not complete:

## Implementation Notes

## Validation Evidence

## Review Notes

## Human Review Request

## Handoff Rules
- Update this card before changing status.
- Attach the active Herdr pane before work starts.
- Move to reviewing only after validation evidence is recorded, the work is committed and pushed to `branch_name` at `commit_sha`, and worker assignment metadata is cleared.
```

## Coordinator Hooks

- `kanban.card.created`: classify readiness and sequencing.
- `kanban.card.updated`: reevaluate routing.
- `kanban.card.status_changed`: dispatch next role.
- `kanban.card.attached`: mark ownership established.
- `kanban.card.detached`: sweep or reassign.
- `pane.agent_status_changed`: compare pane state with card state.
- `pane.closed`: requeue, detach, or mark blocked only when manual intervention is required.
- `coordinator.tick`: fallback stale-state audit.

## Worker Hooks

- `worker.assigned`: read the already attached card.
- `worker.claimed`: move to `ongoing`.
- `worker.progress`: update card only for meaningful checkpoints.
- `worker.blocked`: write concrete blocker and move to `blocked` only when human/manual intervention is required.
- `worker.validation_complete`: record evidence.
- `worker.finished`: commit and push the task branch, record `branch_name` and `commit_sha`, clear assignment metadata, then move to `reviewing`.

## Reviewer Hooks

- `reviewer.assigned`: inspect the `reviewing` card from the already attached reviewer pane.
- `reviewer.accepted`: merge or fast-forward the reviewed branch, push the integration branch when allowed, then move to `done`.
- `reviewer.rejected`: record findings, clear assignment metadata, and move to `todo`.
- `reviewer.invalid_handoff`: record missing branch/commit/evidence, clear assignment metadata, and move to `todo`.
- `reviewer.needs_human_review`: move to `blocked`, set `review_state: human-review-required`, and write the human review request.
- `reviewer.invalid_card`: send back to triage.

## Human Audit Hooks

- `human_audit.assigned`: inspect the already attached blocked card.
- `human_audit.context_loaded`: read card, blocked reason, review notes, human request, branch/commit, validation, and relevant local files before asking.
- `human_audit.checklist_created`: snapshot every original blocker and ambiguity into `## Human Audit Checklist`.
- `human_audit.question`: ask one focused question at a time with a recommended answer.
- `human_audit.followup_created`: create child `clarify:` blocker cards immediately when separate grilling is needed; default children are not parent-blocking.
- `human_audit.resolved`: write `## Human Audit Decision`, run brushup handoff, clear assignment, move the parent to `reviewing` or `todo`, detach, and exit.
- `human_audit.unresolved`: keep the parent `blocked`, keep the audit assignment, and record the unresolved checklist item.

## Sweeper Hooks

- `sweeper.tick`: audit the board.
- `pane.closed`: clear or reassign dead ownership.
- `agent.idle_timeout`: inspect possibly stale work.
- `card.stale_timeout`: mark stale or requeue.
- `card.invariant_failed`: normalize state.
