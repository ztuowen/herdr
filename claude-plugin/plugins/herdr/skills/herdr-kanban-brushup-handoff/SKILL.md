---
name: herdr-kanban-brushup-handoff
description: Use after human audit has resolved a blocked Herdr Kanban card; convert the decision into clean card state, verify original blockers are addressed, create or link follow-up blocker cards, clear audit assignment, move the parent back to reviewing when appropriate, and detach so the sweeper can clean the audit pane.
---

# Herdr Kanban Brushup Handoff

Use this only after `herdr-kanban-human-audit` has produced a decision.

## Preconditions

Do not move the parent out of `blocked` unless:

- `## Human Audit Checklist` exists.
- Every original checklist item is checked or explicitly superseded by a recorded decision.
- `## Human Audit Decision` explains the chosen resolution.
- Any follow-up clarification cards are listed under `## Follow-Up Clarification Cards`.
- Any `parent_blocking: true` child blocker is already resolved or the parent remains blocked.

If any precondition fails, keep the card blocked and return to human audit.

## Brushup

Update the parent card so the next reviewer or worker can act without reading chat history:

1. Summarize the resolved decision in `## Human Audit Decision`.
2. Add implementation or review guidance to `## Review Notes` or `## Implementation Notes`.
3. Ensure acceptance criteria still match the decision; if not, update the card body.
4. Preserve links to child follow-up blockers.
5. Clear frontmatter assignment and blocker fields:

```yaml
owner_role:
assigned_pane:
assigned_workspace:
blocked_reason:
review_state:
last_actor: human-audit
```

6. Move the parent back to review:

```bash
herdr kanban update <uuid> --status reviewing
herdr kanban detach <uuid>
```

7. Exit the session so the sweeper can close the pane.

If the audit decision requires implementation changes instead of review, move the parent to `todo` rather than `reviewing`, and explain why in `## Human Audit Decision`.
