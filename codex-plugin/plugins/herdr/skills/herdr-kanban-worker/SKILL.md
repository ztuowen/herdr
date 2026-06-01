---
name: herdr-kanban-worker
description: Use when an agent is assigned a Herdr Kanban card to implement routine work after triage. The worker verifies the launcher-attached card, executes the card scope, records validation evidence, and moves the card to review or records a blocker.
---

# Herdr Kanban Worker

Read `../../references/kanban-workflow.md` before claiming a card.

Workers do not clarify with the human and do not research beyond the card's stated scope except for local implementation context needed to complete it. If the card is incomplete, record the blocker on the card and stop; move to `blocked` only when human or manual intervention is required.

## Claim

```bash
herdr kanban update <uuid> --status ongoing
```

The dispatcher attaches the launched pane before starting the worker. Do not claim more than one card.

## Execute

1. Read the card contract.
2. Create or switch to the card's task branch/worktree according to repo rules.
3. Inspect only the necessary local context.
4. Implement the scoped change.
5. Run the requested validation, using repo defaults when the card allows it.
6. Update the card description with implementation notes and validation evidence.
7. Commit the completed work on the task branch.
8. Push the task branch when the repo allows it.
9. Record `branch_name` and `commit_sha` in the description front matter.
10. Clear `owner_role`, `assigned_pane`, and `assigned_workspace` before moving the card to review.

Do not move a card to review while the implementation is only a dirty working tree or an unpushed local commit. If repo rules or credentials block commit/push, record the blocker and move the card to `blocked`.

## Finish

Move to review only after validation evidence is recorded and the reviewed work is committed and pushed:

```bash
herdr kanban update <uuid> --status reviewing
```

The worker must unassign before review handoff. Reviewer dispatch owns assigning the next pane.

If blocked by missing human input, approval, or manual intervention, keep the board routable by updating the description with a concrete blocker and moving the card to `blocked`. Do not ask the human directly unless triage delegated that action.

After moving the card to `reviewing` or `blocked`, exit the Codex session. The dispatcher and sweeper own the next pane lifecycle step.
