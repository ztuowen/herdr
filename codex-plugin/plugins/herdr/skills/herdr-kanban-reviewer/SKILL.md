---
name: herdr-kanban-reviewer
description: Use when reviewing Herdr Kanban cards in reviewing status, meaning ready for or already owned by a review agent; validate implementation against acceptance criteria, return failed work with findings, mark accepted work done, or route subjective UI/interface presentation to human review.
---

# Herdr Kanban Reviewer

Read `../../references/kanban-workflow.md` before reviewing cards.

Review against the card contract. If the card is underspecified or wrong, send it back to triage. Do not invent unstated requirements during routine review.

## Workflow

1. List `reviewing` cards, which are ready for or already owned by a review agent.
2. Inspect the card, attached pane, pushed branch, implementation diff, and validation evidence.
3. Compare results to acceptance criteria.
4. Update the card with review notes.
5. Route the card.

## Outcomes

Accepted:

```bash
# Merge or fast-forward the reviewed branch according to repo rules first.
herdr kanban update <uuid> --status done
```

The reviewer is the merge gate. Do not mark accepted work `done` until the reviewed branch has been merged or fast-forwarded into the repo's integration branch and pushed when repo rules allow it.

Rejected:

- record concrete findings in `## Review Notes`
- clear `owner_role`, `assigned_pane`, and `assigned_workspace` metadata if present
- move to `todo` for a fresh worker assignment
- do not move rejected work to `ongoing`

Invalid handoff:

- if `branch_name`, `commit_sha`, validation evidence, or a clean pushed branch is missing, record the missing handoff item in `## Review Notes`
- clear worker assignment metadata and move to `todo`

Invalid card:

- add `clarify:` context or move to `blocked` when human/manual intervention is required
- route back to triage

Human UI/interface review:

- move the card to `blocked`
- add a `review_state: human-review-required` metadata line
- describe what the human must inspect

This path is valid when code-level review passes but final presentation, interaction feel, visual polish, copy, or interface behavior needs human judgment.

## Next Ask Handoff

If review needs human input, do not ask the human directly. Update the card description instead:

- set `review_state: human-review-required` when final acceptance needs human judgment
- set `review_state: triage-question-required` when the card is underspecified, acceptance criteria are wrong, or a product/design ambiguity needs triage
- set `blocked_reason` with a short concrete reason
- fill `## Human Review Request` with the exact question, options, or artifact to inspect
- move the card to `blocked`

Triage or the coordinator owns surfacing that ask to the human and making the card routable again.

After moving the card to `done`, `todo`, or `blocked`, exit the Codex session. The dispatcher and sweeper own the next pane lifecycle step.
