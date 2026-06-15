---
name: herdr-kanban-brushup-handoff
description: Use after human audit has resolved a blocked Herdr Kanban card; convert the decision into clean card state, verify original blockers are addressed, clear audit assignment, move the parent back to reviewing, and detach so the sweeper can clean the audit pane.
---

# Herdr Kanban Brushup Handoff

Use this only after `herdr-kanban-human-audit` has produced a decision.

## Preconditions

Do not move the parent out of `blocked` unless:

- `## Human Audit Checklist` exists.
- Every original checklist item is checked or explicitly superseded by a recorded decision.
- `## Human Audit Decision` explains the chosen resolution.

If any precondition fails, keep the card blocked and return to human audit.

Do not create cards here. Any follow-up the human asked for is recorded as text under `## Follow-Up Requests` and re-enters via triage on a later orchestrator pass.

## Brushup

Update the parent card so the next reviewer or worker can act without reading chat history:

1. Summarize the resolved decision in `## Human Audit Decision`.
2. Add implementation or review guidance to `## Review Notes` or `## Implementation Notes`.
3. Ensure acceptance criteria still match the decision; if not, update the card body.
4. Preserve any `## Follow-Up Requests` text for triage to pick up later.
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

7. Stop working so the sweeper can close the idle pane.

Always move the resolved parent to `reviewing` — never to `todo` or `done` directly. The reviewer owns the next routing decision: accept → `done`, or send back → `todo` for a fresh worker. If the audit decided implementation changes are needed, record that in `## Human Audit Decision` / `## Review Notes` so the reviewer routes it to `todo`; do not route there yourself.
