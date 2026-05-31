---
name: herdr-kanban-sweeper
description: Use when auditing and normalizing Herdr Kanban workflow state, including stale cards, dead pane attachments, blocked work, missing validation evidence, duplicate cards, and routine workflow invariant failures.
---

# Herdr Kanban Sweeper

Read `../../references/kanban-workflow.md` before changing board state.

The sweeper keeps automation honest. It does not clarify with the human and does not complete implementation work.

## Hooks

Run on:

- `sweeper.tick`
- `pane.closed`
- `agent.idle_timeout`
- `card.stale_timeout`
- `card.invariant_failed`
- relevant `kanban.card.updated` events

## Checks

- `ongoing` cards have attached panes.
- attached panes still exist.
- `reviewing` cards contain validation evidence and are ready for or already owned by a review agent.
- `done` cards contain review or human acceptance evidence.
- `blocked` cards require human input, human review, or other manual intervention and are not dispatched to workers.
- `clarify:` cards are not repeatedly assigned.
- duplicate cards or conflicting ownership are flagged.

## Actions

Use card updates to make state explicit. Detach dead panes when needed. Requeue stale routine work only when the card remains ready. Route ambiguous cards back to triage instead of guessing.
