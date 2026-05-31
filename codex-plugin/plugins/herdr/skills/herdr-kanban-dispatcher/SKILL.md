---
name: herdr-kanban-dispatcher
description: Use when routing ready Herdr Kanban cards to worker or reviewer agents, maintaining one active card per agent, and driving the routine post-triage workflow from Herdr board state.
---

# Herdr Kanban Dispatcher

Read `../../references/kanban-workflow.md` before routing cards.

The dispatcher does not clarify, research, or reinterpret card scope. If a card is not routine-ready, send it back to triage by updating the card with the blocker.

## Event Loop

React to these Herdr-native events when available, and use a periodic tick as fallback:

- `kanban.card.created`
- `kanban.card.updated`
- `kanban.card.status_changed`
- `kanban.card.attached`
- `kanban.card.detached`
- `pane.agent_status_changed`
- `pane.closed`
- `coordinator.tick`

## Routing

1. Run `herdr kanban list`.
2. Route `reviewing` cards to reviewer agents first.
3. Route ready `todo` cards to idle worker agents.
4. Surface `blocked` cards with `review_state: human-review-required` to the human.
5. Do not assign `blocked` or `clarify:` cards to workers.
6. Enforce one active card per agent.

Send the selected card UUID to the target agent and instruct it to use the matching role skill. The assigned agent must attach its own pane with `herdr kanban attach <uuid>`.

## Invariants

- Work starts only from a card.
- Ownership is represented by an attached Herdr pane.
- Handoff context lives in the card description, not in chat history.
- The dispatcher may requeue stale or invalid work but must not silently change scope.
