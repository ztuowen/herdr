# Herdr Kanban Workflow Contract

Herdr Kanban is the only handoff surface. Agents must not rely on chat history, private notes, or implicit context after triage. A downstream agent should be able to resume from:

- Kanban card title
- Kanban card description
- Kanban status
- attached pane
- Herdr-visible agent status

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

`herdr kanban attach` must be run inside the pane that owns the work.

## Status Semantics

- `todo`: ready routine work, unless prefixed with `clarify:`.
- `ongoing`: claimed by an attached worker pane.
- `blocked`: waiting on human input, human review, or other manual intervention.
- `reviewing`: ready for or already owned by a review agent after implementation and validation evidence are recorded.
- `done`: accepted by reviewer or human.

Because the current Herdr Kanban status set has no dedicated `human-review` column, encode human presentation review as `blocked` plus metadata.

## Title Prefixes

- `clarify:` means triage still owns clarification or research.

## Card Template

```markdown
---
workflow_state: ready
owner_role:
assigned_pane:
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

## Implementation Notes

## Validation Evidence

## Review Notes

## Human Review Request

## Handoff Rules
- Update this card before changing status.
- Attach the active Herdr pane before work starts.
- Move to reviewing only after validation evidence is recorded.
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

- `worker.assigned`: read card and claim it.
- `worker.claimed`: move to `ongoing`.
- `worker.progress`: update card only for meaningful checkpoints.
- `worker.blocked`: write concrete blocker and move to `blocked` only when human/manual intervention is required.
- `worker.validation_complete`: record evidence.
- `worker.finished`: move to `reviewing`.

## Reviewer Hooks

- `reviewer.assigned`: inspect a `reviewing` card.
- `reviewer.accepted`: move to `done`.
- `reviewer.rejected`: record findings and return.
- `reviewer.needs_human_review`: move to `blocked`, set `review_state: human-review-required`, and write the human review request.
- `reviewer.invalid_card`: send back to triage.

## Sweeper Hooks

- `sweeper.tick`: audit the board.
- `pane.closed`: clear or reassign dead ownership.
- `agent.idle_timeout`: inspect possibly stale work.
- `card.stale_timeout`: mark stale or requeue.
- `card.invariant_failed`: normalize state.
