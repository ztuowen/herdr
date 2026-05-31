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

- `in-progress` cards have attached panes.
- attached panes still exist.
- `need-review` cards contain validation evidence.
- `done` cards contain review or human acceptance evidence.
- `human-review:` cards stay in `need-review` and are not dispatched to workers.
- `blocked:` and `clarify:` cards are not repeatedly assigned.
- duplicate cards or conflicting ownership are flagged.

## Actions

Use card updates to make state explicit. Detach dead panes when needed. Requeue stale routine work only when the card remains ready. Route ambiguous cards back to triage instead of guessing.
