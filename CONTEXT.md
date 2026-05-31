# herdr Context

Domain language for Herdr's terminal workspace manager and mouse-first TUI.

## Language

**Markdown preview**:
The derived, render-ready view of a Markdown document for a terminal area. It owns wrapping, scroll bounds, active link positions, math image placements, and scrollbar allocation, but not input handling or platform side effects.
_Avoid_: Markdown viewport, markdown renderer, markdown panel

**Kanban board projection**:
The derived, render-ready view of the Kanban board for a terminal area and layout mode. It owns column/row geometry, visible card positions, scroll offsets, hit targets, and selection clamping, but not item mutation, pane focusing, persistence, or drawing.
The app does not need to know which item is selected; callers ask the Kanban board module for a board action rather than reconstructing selection from columns, rows, layout mode, and scroll offsets.
Left-clicking a tracked card is an app policy: the board module reports card activation with the item identity and terminal link, while the app decides whether live pane state means focus the pane or open card details.
Pure Kanban board changes mutate `KanbanState` inside the board module. Effects that require app side effects are returned as board actions for the app to apply.
_Avoid_: Kanban viewport, board renderer, kanban layout helper
