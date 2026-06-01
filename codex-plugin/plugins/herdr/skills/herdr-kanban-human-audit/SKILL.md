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

Keep asking until every original checklist item is addressed. If a new ambiguity is directly downstream of the same decision, keep the same audit session open and continue. If the new ambiguity is separate follow-up work, create a child blocker card immediately.

## Follow-Up Blockers

Create follow-up cards on the spot when separate grilling or triage is needed. Use `clarify:` titles and seed them with enough context for a new audit session.

Default follow-up cards are not parent-blocking. Mark `parent_blocking: true` only when the parent cannot move forward without that child decision.

Record created children on the parent under:

```markdown
## Follow-Up Clarification Cards
- uuid: <child-uuid>
  title: clarify: ...
  parent_blocking: false
  seed_context: ...
```

## Resolve

The parent can leave `blocked` only when all original `## Human Audit Checklist` items are addressed.

When resolved:

1. Write `## Human Audit Decision`.
2. Create any follow-up blocker cards and link them.
3. Use the `herdr-kanban-brushup-handoff` skill to make the parent routable.

If unresolved, keep the card `blocked`, keep `owner_role: human-audit`, keep `assigned_pane`, and record the exact unresolved checklist item. Do not detach.
