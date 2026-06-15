---
name: herdr-kanban-human-audit
description: Use when a blocked Herdr Kanban card needs interactive human clarification, design resolution, or triage-question resolution before release; inspect all available context first, grill one focused question at a time, record decisions, and keep the card blocked until every original blocker is addressed.
---

# Herdr Kanban Human Audit

Read `../../references/kanban-workflow.md` before changing board state.

The human-audit pane is already attached by the launcher. Do not detach or clear assignment until the parent card is resolved or explicitly handed off.

## Inspect First

Before asking the human anything:

1. Read the card description and frontmatter.
2. Read `blocked_reason`, `review_state`, `## Review Notes`, and `## Human Review Request`.
3. Read `branch_name`, `commit_sha`, validation evidence, and relevant diffs when present.
4. Inspect nearby docs, code, screenshots, or artifacts needed to understand the decision.
5. Create or update `## Human Audit Checklist` so every original blocker, review ambiguity, and human request is represented as a checkbox.

Do not rely on chat history as the source of truth. Put the audit state in the card.

## Interview

Ask one focused question at a time. Each question must include:

- the specific checklist item it addresses
- the relevant context you inspected
- your recommended answer
- the tradeoff if the human chooses differently

Keep asking until every original checklist item is addressed. If a new ambiguity is directly downstream of the same decision, keep the same audit session open and continue.

## Follow-Up Requests

Do not create cards. When the human asks for separate follow-up work during the audit, record the request as text on the parent under `## Follow-Up Requests` — do not spawn `clarify:` or child blocker cards on the spot. Follow-up work re-enters the pipeline through triage on a later orchestrator pass, not from this audit session.

```markdown
## Follow-Up Requests
- <what the human asked for, with enough context for triage to pick it up later>
```

This keeps the card on the single forward path (human-review → review → done, or human-review → review → todo → worker) instead of fanning out new tickets mid-audit.

## Resolve

The parent can leave `blocked` only when all original `## Human Audit Checklist` items are addressed.

When resolved:

1. Write `## Human Audit Decision`.
2. Record any follow-up the human asked for under `## Follow-Up Requests` (text only — no new cards).
3. Use the `herdr-kanban-brushup-handoff` skill to move the parent to `reviewing`.

Always hand the resolved parent back to `reviewing`. Never move it to `todo`, `done`, or any other status directly — the reviewer owns the next routing decision (accept → done, or send → todo for a fresh worker).

If unresolved, keep the card `blocked`, keep `owner_role: human-audit`, keep `assigned_pane`, and record the exact unresolved checklist item. Do not detach.
