---
name: herdr-kanban-reviewer
description: Use when reviewing Herdr Kanban cards in need-review, validating implementation against card acceptance criteria, returning failed work with findings, marking accepted work done, or routing subjective UI/interface presentation to human review.
---

# Herdr Kanban Reviewer

Read `../../references/kanban-workflow.md` before reviewing cards.

Review against the card contract. If the card is underspecified or wrong, send it back to triage. Do not invent unstated requirements during routine review.

## Workflow

1. List `need-review` cards.
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
- keep or return to `in-progress` only when the same worker should continue

Invalid card:

- add `clarify:` or `blocked:` context
- route back to triage

Human UI/interface review:

- keep the card in `need-review`
- prefix the title with `human-review:`
- add a `review_state: human-review-required` metadata line
- describe what the human must inspect

This path is valid when code-level review passes but final presentation, interaction feel, visual polish, copy, or interface behavior needs human judgment.
