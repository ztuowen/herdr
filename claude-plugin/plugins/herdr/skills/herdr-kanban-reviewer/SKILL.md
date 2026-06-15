---
name: herdr-kanban-reviewer
description: Use when reviewing Herdr Kanban cards in reviewing status, meaning ready for or already owned by a review agent; validate implementation against acceptance criteria, return failed work with findings, mark accepted work done, or route subjective UI/interface presentation to human review.
---

# Herdr Kanban Reviewer

Read `../../references/kanban-workflow.md` before reviewing cards.

Review against the card contract. If the card is underspecified or wrong, send it back to triage. Do not invent unstated requirements during routine review.

## Workflow

1. List `reviewing` cards, which are ready for or already owned by a review agent.
2. Confirm the dispatcher-attached review pane owns the card.
3. Inspect the card, attached pane, pushed branch, implementation diff, and validation evidence.
4. Compare results to acceptance criteria.
5. Update the card with review notes.
6. Route the card.

## Design Review

Beyond acceptance criteria, review *how* the diff is built, not just whether it passes. Flagging a design smell — duplication, a growing central switch, misplaced policy, hand-maintained invariants — is not inventing a requirement; record findings in `## Review Notes` and reject when a clearly cleaner form exists.

Distinguish a design smell (fair to send back) from an unstated feature (do not invent — route to triage). A smell that only appears when a pattern repeats across many cards is out of single-card scope; note it and route to triage.

## Outcomes

**Ordering invariant — status change is your last action.** Integration is part of the reviewer's job, not a step after handoff. Do every part of an outcome first (review, and for acceptance the full integrate + push + cleanup), and only then change the card status. The status change is the single final action, followed by clearing assignment. While the card still reads `reviewing`, your `owner_role`/`assigned_pane` are the only thing stopping the dispatcher from routing a second reviewer — so never clear assignment, detach, or leave the session while it still reads `reviewing`. Run the whole outcome to completion in one session; do not flip status partway.

Accepted — do these in order, status second-to-last:

1. Confirm the work meets acceptance criteria and validation evidence.
2. Integrate per repo rules: merge/squash-merge the task branch into the base branch from the shared checkout, push when allowed (see Integration below).
3. Run repo cleanup (remove the task worktree, delete the task branch).
4. Record the integration result in `## Review Notes`.
5. Only after integration + push + cleanup succeed, mark it done — this is the final status change:

```bash
herdr kanban update <uuid> --status done
```

6. Clear `owner_role`, `assigned_pane`, and `assigned_workspace`.

The reviewer is the merge gate. If integration or push fails or is blocked, do **not** mark `done`: leave the card `reviewing` (or move to `blocked` with a concrete `blocked_reason` if human/manual intervention is needed) and record what failed. Never reach `done` on an unintegrated branch.

## Integration

Follow the repo isolation policy (`CLAUDE.md`, `AGENT.md`, or `AGENTS.md`) if present. When the policy defines an integrate step (e.g. squash-merge the task branch into the base branch from the shared checkout, format, push), perform it from the shared checkout — not from the worker's task worktree. After a successful integrate, run the policy's cleanup (remove the task worktree, delete the task branch locally and remotely) so disposable worktrees do not accumulate. Record the integration result in `## Review Notes`. With no policy, merge or fast-forward into the integration branch and push when allowed.

Rejected:

- record concrete findings in `## Review Notes`
- move to `todo` first (`herdr kanban update <uuid> --status todo`)
- then clear `owner_role`, `assigned_pane`, and `assigned_workspace` metadata
- do not move rejected work to `ongoing`

Invalid handoff:

- if `branch_name`, `commit_sha`, validation evidence, or a clean pushed branch is missing, record the missing handoff item in `## Review Notes`
- move to `todo` first, then clear worker assignment metadata

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
- move the card to `blocked` (do this before clearing assignment)
- only after it reads `blocked`, clear `owner_role`, `assigned_pane`, and `assigned_workspace` so the human-audit launcher can claim it

Triage or the coordinator owns surfacing that ask to the human and making the card routable again.

After moving the card to `done`, `todo`, or `blocked`, stop working and leave the session idle. The dispatcher and sweeper own the next pane lifecycle step — the sweeper reclaims the pane once it reads as idle.
