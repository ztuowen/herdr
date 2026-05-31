---
name: herdr-kanban-reviewer
description: Use when reviewing Herdr Kanban cards in reviewing status, meaning ready for or already owned by a review agent; validate implementation against acceptance criteria, return failed work with findings, mark accepted work done, or route subjective UI/interface presentation to human review.
---

# Herdr Kanban Reviewer

Read `../../references/kanban-workflow.md` before reviewing cards.

Review against the card contract. If the card is underspecified or wrong, send it back to triage. Do not invent unstated requirements during routine review.

## Workflow

1. List `reviewing` cards, which are ready for or already owned by a review agent.
2. Inspect the card, attached pane, implementation diff, and validation evidence.
3. Compare results to acceptance criteria.
4. Update the card with review notes.
5. Route the card.

## Outcomes

Accepted:

```bash
herdr kanban update <uuid> --status done
```

Rejected:

- record findings in the card
- move to `todo` when worker reassignment is enough
- keep or return to `ongoing` only when the same worker should continue

Invalid card:

- add `clarify:` context or move to `blocked` when human/manual intervention is required
- route back to triage

Human UI/interface review:

- move the card to `blocked`
- add a `review_state: human-review-required` metadata line
- describe what the human must inspect

This path is valid when code-level review passes but final presentation, interaction feel, visual polish, copy, or interface behavior needs human judgment.
