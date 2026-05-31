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
herdr kanban list --status need-review
herdr kanban update <uuid> --title "<title>"
herdr kanban update <uuid> --description <path.md>
herdr kanban update <uuid> --status <todo|in-progress|need-review|done>
herdr kanban attach <uuid>
herdr kanban detach <uuid>
```

`herdr kanban attach` must be run inside the pane that owns the work.

## Status Semantics

- `todo`: ready routine work, unless prefixed with `blocked:` or `clarify:`.
- `in-progress`: claimed by an attached worker pane.
- `need-review`: implementation complete, evidence recorded, waiting for review or human UI/interface review.
- `done`: accepted by reviewer or human.

Because the current Herdr Kanban status set has no `blocked` or `human-review` columns, encode those states in title prefixes and metadata.

## Title Prefixes

- `blocked:` means do not dispatch until triage resolves the blocker.
- `clarify:` means triage still owns clarification or research.
- `human-review:` means reviewer accepted the routine checks but human presentation review is required.

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
- Move to need-review only after validation evidence is recorded.
```

## Coordinator Hooks

- `kanban.card.created`: classify readiness and sequencing.
- `kanban.card.updated`: reevaluate routing.
- `kanban.card.status_changed`: dispatch next role.
- `kanban.card.attached`: mark ownership established.
- `kanban.card.detached`: sweep or reassign.
- `pane.agent_status_changed`: compare pane state with card state.
- `pane.closed`: requeue, detach, or mark blocked.
- `coordinator.tick`: fallback stale-state audit.

## Worker Hooks

- `worker.assigned`: read card and claim it.
- `worker.claimed`: move to `in-progress`.
- `worker.progress`: update card only for meaningful checkpoints.
- `worker.blocked`: write concrete blocker and stop.
- `worker.validation_complete`: record evidence.
- `worker.finished`: move to `need-review`.

## Reviewer Hooks

- `reviewer.assigned`: inspect a `need-review` card.
- `reviewer.accepted`: move to `done`.
- `reviewer.rejected`: record findings and return.
- `reviewer.needs_human_review`: keep in `need-review`, prefix `human-review:`, and write the human review request.
- `reviewer.invalid_card`: send back to triage.

## Sweeper Hooks

- `sweeper.tick`: audit the board.
- `pane.closed`: clear or reassign dead ownership.
- `agent.idle_timeout`: inspect possibly stale work.
- `card.stale_timeout`: mark stale or requeue.
- `card.invariant_failed`: normalize state.
