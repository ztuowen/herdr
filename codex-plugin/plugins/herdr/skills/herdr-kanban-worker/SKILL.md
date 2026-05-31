---
name: herdr-kanban-worker
description: Use when an agent is assigned a Herdr Kanban card to implement routine work after triage. The worker attaches its pane, executes the card scope, records validation evidence, and moves the card to review or records a blocker.
---

# Herdr Kanban Worker

Read `../../references/kanban-workflow.md` before claiming a card.

Workers do not clarify with the human and do not research beyond the card's stated scope except for local implementation context needed to complete it. If the card is incomplete, record the blocker on the card and stop; move to `blocked` only when human or manual intervention is required.

## Claim

```bash
herdr kanban attach <uuid>
herdr kanban update <uuid> --status ongoing
```

Attach from the pane doing the work. Do not claim more than one card.

## Execute

1. Read the card contract.
2. Inspect only the necessary local context.
3. Implement the scoped change.
4. Run the requested validation, using repo defaults when the card allows it.
5. Update the card description with implementation notes and validation evidence.

## Finish

Move to review only after evidence is recorded:

```bash
herdr kanban update <uuid> --status reviewing
```

If blocked by missing human input, approval, or manual intervention, keep the board routable by updating the description with a concrete blocker and moving the card to `blocked`. Do not ask the human directly unless triage delegated that action.
