---
name: herdr-kanban-triage
description: Use when converting a user request, bug report, issue, or research question into ready Herdr Kanban cards. This skill owns clarification, research, decomposition, acceptance criteria, dependency ordering, and human questions before routine agents take over.
---

# Herdr Kanban Triage

Read `../../references/kanban-workflow.md` before creating or updating cards.

Triage is the only phase that should clarify with the human or perform open-ended research. After triage, worker, reviewer, dispatcher, and sweeper agents must be able to operate from Herdr-native state alone.

## Workflow

1. Clarify the user request until routine execution would be safe.
2. Research local repo context, linked issues, docs, errors, or prior work as needed.
3. Split the work into independently reviewable cards.
4. Create only ready cards, or explicitly mark unresolved work as `clarify:` or move it to `blocked` when human or manual intervention is required.
5. Put the full handoff contract in each card description.

Use:

```bash
herdr kanban add "<action-oriented title>" --description <path.md> --status todo
```

## Card Requirements

Every ready card must include:

- objective
- context
- scope
- out of scope
- acceptance criteria
- validation requirements
- dependencies
- handoff rules

Do not create vague cards. If the task cannot be made routine, keep ownership in triage and ask the human.

## Status Rules

- `todo`: ready for dispatcher and worker automation.
- `ongoing`: only after a worker has attached a pane.
- `blocked`: waiting on human input, human review, or other manual intervention.
- `reviewing`: ready for or already owned by a review agent after implementation and validation evidence are recorded.
- `done`: only after reviewer acceptance or explicit human acceptance.

For UI/interface work, include the expected human review path when final presentation cannot be judged automatically. Use `blocked` with `review_state: human-review-required` when the code review passed but the next action is human presentation review.
