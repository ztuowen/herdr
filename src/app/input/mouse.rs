use bytes::Bytes;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Direction, Rect};
use tracing::warn;

use crate::{
    app::state::{
        AgentPanelScope, AppState, ContextMenuKind, ContextMenuState, DragState, DragTarget,
        MenuListState, Mode, TabPressState, ViewLayout, WorkspacePressState,
    },
    layout::{PaneInfo, SplitBorder},
    selection::Selection,
    terminal::TerminalRuntimeRegistry,
};

#[cfg(test)]
use super::WheelRouting;
use super::{
    modal::{
        apply_context_menu_action, apply_global_menu_action, apply_rename_action,
        confirm_close_accept, confirm_close_cancel, global_menu_actions, leave_modal,
        modal_action_from_buttons, open_global_menu, open_new_tab_dialog, ModalAction,
    },
    settings::SettingsAction,
    ScrollbarClickTarget, TAB_DRAG_THRESHOLD, WORKSPACE_DRAG_THRESHOLD,
};

impl AppState {
    pub(crate) fn handle_pane_mouse_only(
        &mut self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        mouse: MouseEvent,
    ) {
        if self.mode != Mode::Terminal {
            return;
        }
        let Some(info) = self.pane_at(mouse.column, mouse.row).cloned() else {
            return;
        };

        match mouse.kind {
            MouseEventKind::ScrollUp
            | MouseEventKind::ScrollDown
            | MouseEventKind::ScrollLeft
            | MouseEventKind::ScrollRight => {
                self.forward_pane_reported_wheel(terminal_runtimes, &info, mouse);
            }
            MouseEventKind::Down(_) | MouseEventKind::Up(_) | MouseEventKind::Drag(_) => {
                self.forward_pane_mouse_button(terminal_runtimes, &info, mouse);
            }
            MouseEventKind::Moved => {}
        }
    }

    pub(super) fn handle_mouse(
        &mut self,
        terminal_runtimes: &mut TerminalRuntimeRegistry,
        mouse: MouseEvent,
    ) -> Option<SettingsAction> {
        if self.view.layout == ViewLayout::Mobile
            && self.mode != Mode::Navigate
            && matches!(self.mode, Mode::Terminal | Mode::Resize | Mode::Kanban)
            && rect_contains(self.view.mobile_menu_hit_area, mouse.column, mouse.row)
        {
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                self.mobile_switcher_scroll = 0;
                self.mode = Mode::Navigate;
            }
            return None;
        }

        if self.mode == Mode::Onboarding {
            self.handle_onboarding_mouse(mouse);
            return None;
        }

        if self.mode == Mode::Kanban {
            if self.kanban_detail_uuid.is_some() {
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        let (width, height) = self.kanban_detail_modal_size();
                        let popup =
                            crate::ui::centered_popup_rect(self.view.terminal_area, width, height);
                        if let Some(rect) = popup {
                            let inside = mouse.column >= rect.x
                                && mouse.column < rect.x + rect.width
                                && mouse.row >= rect.y
                                && mouse.row < rect.y + rect.height;
                            if !inside {
                                self.set_kanban_detail_uuid(None);
                            } else if rect.height >= 9 {
                                let inner = Rect::new(
                                    rect.x + 1,
                                    rect.y + 1,
                                    rect.width.saturating_sub(2),
                                    rect.height.saturating_sub(2),
                                );
                                let rows = ratatui::layout::Layout::vertical([
                                    ratatui::layout::Constraint::Length(1), // Header title
                                    ratatui::layout::Constraint::Length(1), // Divider
                                    ratatui::layout::Constraint::Length(1), // Card Title
                                    ratatui::layout::Constraint::Length(1), // Status Badge
                                    ratatui::layout::Constraint::Length(1), // UUID
                                    ratatui::layout::Constraint::Length(1), // Associated Terminal
                                    ratatui::layout::Constraint::Min(1),    // Description
                                    ratatui::layout::Constraint::Length(1), // Footer hint
                                ])
                                .split(inner);

                                // Check if user clicked on any markdown hyperlink
                                if let Some((_, _, url)) = self
                                    .active_kanban_detail_hyperlinks()
                                    .into_iter()
                                    .find(|((x, y), _, _)| *x == mouse.column && *y == mouse.row)
                                {
                                    if !url.starts_with("http://") && !url.starts_with("https://") {
                                        self.request_clipboard_write = Some(url.into_bytes());
                                        return None;
                                    }
                                }

                                // Check if user clicked on the terminal row (rows[5])
                                let terminal_row = rows[5];
                                if mouse.row == terminal_row.y {
                                    if let Some(ref uuid) = self.kanban_detail_uuid {
                                        if let Some(item) =
                                            self.kanban_items.iter().find(|it| it.uuid == *uuid)
                                        {
                                            if let Some(ref tid) = item.terminal_id {
                                                let status = self.kanban_item_pane_status(tid);
                                                if status.exists {
                                                    if let Some((ws_idx, tab_idx, pane_id)) =
                                                        self.find_pane_by_terminal_id_str(tid)
                                                    {
                                                        self.focus_navigator_target(
                                                            crate::app::state::NavigatorTarget::Pane {
                                                                ws_idx,
                                                                tab_idx,
                                                                pane_id,
                                                            },
                                                        );
                                                        self.set_kanban_detail_uuid(None);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                let footer_row = rows[7];
                                if mouse.row == footer_row.y {
                                    let text_len = 54;
                                    let x_start = footer_row.x
                                        + (footer_row.width.saturating_sub(text_len)) / 2;
                                    if mouse.column >= x_start && mouse.column < x_start + text_len
                                    {
                                        let offset = mouse.column - x_start;
                                        if offset < 22 {
                                            self.set_kanban_detail_uuid(None);
                                        } else if offset < 38 {
                                            if let Some(ref uuid) = self.kanban_detail_uuid {
                                                self.request_clipboard_write =
                                                    Some(uuid.clone().into_bytes());
                                            }
                                        } else {
                                            self.kanban_delete_selected();
                                            self.set_kanban_detail_uuid(None);
                                        }
                                    }
                                }
                            }
                        } else {
                            self.set_kanban_detail_uuid(None);
                        }
                    }
                    MouseEventKind::ScrollUp => {
                        self.scroll_kanban_detail(-3);
                    }
                    MouseEventKind::ScrollDown => {
                        self.scroll_kanban_detail(3);
                    }
                    _ => {}
                }
                return None;
            }

            let sidebar = self.view.sidebar_rect;
            let in_sidebar = sidebar.width > 0
                && mouse.column >= sidebar.x
                && mouse.column < sidebar.x + sidebar.width
                && mouse.row >= sidebar.y
                && mouse.row < sidebar.y + sidebar.height;

            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left)
                | MouseEventKind::Down(MouseButton::Right)
                    if !in_sidebar =>
                {
                    if let Some((col_idx, row_idx, item)) =
                        self.kanban_item_at(mouse.column, mouse.row)
                    {
                        self.kanban_selected_col = col_idx;
                        self.kanban_selected_row = row_idx;

                        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Right)) {
                            self.set_kanban_detail_uuid(Some(item.uuid.clone()));
                        } else {
                            let mut navigated = false;
                            if let Some(ref term_id_str) = item.terminal_id {
                                if let Some((ws_idx, tab_idx, pane_id)) =
                                    self.find_pane_by_terminal_id_str(term_id_str)
                                {
                                    self.focus_navigator_target(
                                        crate::app::state::NavigatorTarget::Pane {
                                            ws_idx,
                                            tab_idx,
                                            pane_id,
                                        },
                                    );
                                    navigated = true;
                                }
                            }
                            if !navigated {
                                self.set_kanban_detail_uuid(Some(item.uuid.clone()));
                            }
                        }
                    }
                    return None;
                }
                MouseEventKind::ScrollUp | MouseEventKind::ScrollDown if !in_sidebar => {
                    let is_portrait = self.view.layout == ViewLayout::Mobile;
                    let sections = if is_portrait {
                        ratatui::layout::Layout::vertical([
                            ratatui::layout::Constraint::Percentage(25),
                            ratatui::layout::Constraint::Percentage(25),
                            ratatui::layout::Constraint::Percentage(25),
                            ratatui::layout::Constraint::Percentage(25),
                        ])
                        .split(self.view.terminal_area)
                    } else {
                        ratatui::layout::Layout::horizontal([
                            ratatui::layout::Constraint::Percentage(25),
                            ratatui::layout::Constraint::Percentage(25),
                            ratatui::layout::Constraint::Percentage(25),
                            ratatui::layout::Constraint::Percentage(25),
                        ])
                        .split(self.view.terminal_area)
                    };

                    let mut hovered_col = None;
                    for col_idx in 0..4 {
                        let col_area = sections[col_idx];
                        let inside = if is_portrait {
                            mouse.row >= col_area.y && mouse.row < col_area.y + col_area.height
                        } else {
                            mouse.column >= col_area.x && mouse.column < col_area.x + col_area.width
                        };
                        if inside {
                            hovered_col = Some(col_idx);
                            break;
                        }
                    }

                    if let Some(col_idx) = hovered_col {
                        if self.kanban_selected_col != col_idx {
                            self.kanban_selected_col = col_idx;
                            self.kanban_selected_row = 0;
                        }

                        let items_count = self.kanban_items_in_column(col_idx).len();
                        if items_count > 0 {
                            match mouse.kind {
                                MouseEventKind::ScrollUp => {
                                    self.kanban_selected_row =
                                        self.kanban_selected_row.saturating_sub(1);
                                }
                                MouseEventKind::ScrollDown => {
                                    self.kanban_selected_row =
                                        (self.kanban_selected_row + 1).min(items_count - 1);
                                }
                                _ => {}
                            }
                        }
                    }
                    return None;
                }
                _ => {}
            }

            if !in_sidebar {
                return None;
            }
        }

        if self.mode == Mode::Terminal
            && self.clickable_toast_at(mouse.column, mouse.row)
            && matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
        {
            self.focus_toast_target();
            return None;
        }

        if self.mode == Mode::Terminal
            && self.clickable_toast_at(mouse.column, mouse.row)
            && matches!(mouse.kind, MouseEventKind::Up(MouseButton::Left))
        {
            return None;
        }

        if self.mode == Mode::Settings {
            return self.handle_settings_mouse(mouse);
        }

        let launcher_enabled = self.view.layout != ViewLayout::Mobile
            && !self.sidebar_collapsed
            && matches!(
                self.mode,
                Mode::Terminal
                    | Mode::Navigate
                    | Mode::Resize
                    | Mode::GlobalMenu
                    | Mode::KeybindHelp
                    | Mode::Kanban
            );
        let launcher = self.global_launcher_rect();
        let launcher_hit = launcher_enabled
            && mouse.column >= launcher.x
            && mouse.column < launcher.x + launcher.width
            && mouse.row >= launcher.y
            && mouse.row < launcher.y + launcher.height;

        if matches!(mouse.kind, MouseEventKind::Moved) && self.mode == Mode::GlobalMenu {
            let actions = global_menu_actions(self);
            let hovered = self
                .global_menu_item_at(mouse.column, mouse.row)
                .and_then(|action| actions.iter().position(|item| *item == action));
            self.global_menu.hover(hovered);
            return None;
        }

        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) && launcher_hit {
            if self.mode == Mode::GlobalMenu {
                leave_modal(self);
            } else {
                open_global_menu(self);
            }
            return None;
        }

        if self.mode == Mode::GlobalMenu {
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                if let Some(action) = self.global_menu_item_at(mouse.column, mouse.row) {
                    apply_global_menu_action(self, action);
                } else {
                    leave_modal(self);
                }
            }
            return None;
        }

        if self.mode == Mode::KeybindHelp {
            return None;
        }

        if self.view.layout == ViewLayout::Mobile && self.handle_mobile_mouse(mouse) {
            return None;
        }

        let sidebar = self.view.sidebar_rect;
        let in_sidebar = mouse.column >= sidebar.x
            && mouse.column < sidebar.x + sidebar.width
            && mouse.row >= sidebar.y
            && mouse.row < sidebar.y + sidebar.height;

        if self.mode == Mode::OpenExistingWorktree {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    if let Some(open) = &mut self.worktree_open {
                        open.selected = open.selected.saturating_sub(1);
                    }
                    return None;
                }
                MouseEventKind::ScrollDown => {
                    if let Some(open) = &mut self.worktree_open {
                        open.selected =
                            (open.selected + 1).min(open.entries.len().saturating_sub(1));
                    }
                    return None;
                }
                _ => {}
            }
        }

        if matches!(
            self.mode,
            Mode::NewLinkedWorktree | Mode::OpenExistingWorktree | Mode::ConfirmRemoveWorktree
        ) && !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
        {
            return None;
        }

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.selection = None;
                self.selection_autoscroll = None;
                self.workspace_press = None;

                if self.mode == Mode::ConfirmClose {
                    let popup = self.confirm_close_rect();
                    let inner = Rect::new(
                        popup.x + 1,
                        popup.y + 1,
                        popup.width.saturating_sub(2),
                        popup.height.saturating_sub(2),
                    );
                    let (confirm, cancel) = crate::ui::confirm_close_button_rects(inner);
                    match modal_action_from_buttons(
                        mouse.column,
                        mouse.row,
                        &[
                            (confirm, ModalAction::Confirm),
                            (cancel, ModalAction::Cancel),
                        ],
                    ) {
                        Some(ModalAction::Confirm) => confirm_close_accept(self),
                        Some(ModalAction::Cancel) | None => confirm_close_cancel(self),
                        _ => {}
                    }
                    return None;
                }

                if self.mode == Mode::NewLinkedWorktree {
                    if let Some(inner) =
                        crate::ui::new_linked_worktree_inner_rect(self.screen_rect())
                    {
                        let (create, cancel) = crate::ui::new_linked_worktree_button_rects(inner);
                        match modal_action_from_buttons(
                            mouse.column,
                            mouse.row,
                            &[
                                (create, ModalAction::Confirm),
                                (cancel, ModalAction::Cancel),
                            ],
                        ) {
                            Some(ModalAction::Confirm) => {
                                self.request_submit_worktree_create = true;
                            }
                            Some(ModalAction::Cancel)
                                if !self
                                    .worktree_create
                                    .as_ref()
                                    .is_some_and(|create| create.creating) =>
                            {
                                self.worktree_create = None;
                                self.name_input.clear();
                                self.name_input_replace_on_type = false;
                                leave_modal(self);
                            }
                            _ => {}
                        }
                    }
                    return None;
                }

                if self.mode == Mode::OpenExistingWorktree {
                    if let Some(open) = self.worktree_open.as_ref() {
                        if let Some(inner) = crate::ui::open_existing_worktree_inner_rect(
                            self.screen_rect(),
                            open.entries.len(),
                        ) {
                            let max_rows = inner.height.saturating_sub(4) as usize;
                            let start = crate::ui::open_existing_worktree_visible_start(
                                open.selected,
                                max_rows,
                            );
                            let row_idx = if rect_contains(inner, mouse.column, mouse.row) {
                                mouse
                                    .row
                                    .checked_sub(inner.y.saturating_add(2))
                                    .map(usize::from)
                                    .filter(|row| *row < max_rows)
                                    .and_then(|row| {
                                        let entry_idx = start + row;
                                        (entry_idx < open.entries.len()).then_some(entry_idx)
                                    })
                            } else {
                                None
                            };
                            if let Some(entry_idx) = row_idx {
                                if let Some(open) = &mut self.worktree_open {
                                    open.selected = entry_idx;
                                }
                                self.request_submit_worktree_open = true;
                                return None;
                            }

                            let (open_button, cancel) =
                                crate::ui::open_existing_worktree_button_rects(inner);
                            match modal_action_from_buttons(
                                mouse.column,
                                mouse.row,
                                &[
                                    (open_button, ModalAction::Confirm),
                                    (cancel, ModalAction::Cancel),
                                ],
                            ) {
                                Some(ModalAction::Confirm) => {
                                    self.request_submit_worktree_open = true;
                                }
                                Some(ModalAction::Cancel) => {
                                    self.worktree_open = None;
                                    leave_modal(self);
                                }
                                _ => {}
                            }
                        }
                    }
                    return None;
                }

                if self.mode == Mode::ConfirmRemoveWorktree {
                    if let Some(popup) = crate::ui::remove_worktree_popup_rect(self.screen_rect()) {
                        let inner = Rect::new(
                            popup.x + 1,
                            popup.y + 1,
                            popup.width.saturating_sub(2),
                            popup.height.saturating_sub(2),
                        );
                        let force_confirmation = self
                            .worktree_remove
                            .as_ref()
                            .is_some_and(|remove| remove.force_confirmation);
                        let (remove, cancel) =
                            crate::ui::remove_worktree_button_rects(inner, force_confirmation);
                        match modal_action_from_buttons(
                            mouse.column,
                            mouse.row,
                            &[
                                (remove, ModalAction::Confirm),
                                (cancel, ModalAction::Cancel),
                            ],
                        ) {
                            Some(ModalAction::Confirm) => {
                                self.request_submit_worktree_remove = true;
                            }
                            Some(ModalAction::Cancel)
                                if !self
                                    .worktree_remove
                                    .as_ref()
                                    .is_some_and(|remove| remove.removing) =>
                            {
                                self.worktree_remove = None;
                                leave_modal(self);
                            }
                            _ => {}
                        }
                    }
                    return None;
                }

                if matches!(
                    self.mode,
                    Mode::RenameWorkspace | Mode::RenameTab | Mode::RenamePane
                ) {
                    let action = self
                        .rename_modal_inner()
                        .map(crate::ui::rename_button_rects)
                        .and_then(|(save, clear, cancel)| {
                            modal_action_from_buttons(
                                mouse.column,
                                mouse.row,
                                &[
                                    (save, ModalAction::Save),
                                    (clear, ModalAction::Clear),
                                    (cancel, ModalAction::Cancel),
                                ],
                            )
                        })
                        .unwrap_or(ModalAction::Cancel);
                    apply_rename_action(self, action);
                    return None;
                }

                if self.mode == Mode::ContextMenu {
                    let item_idx = self.context_menu_item_at(mouse.column, mouse.row);
                    if let Some(menu) = self.context_menu.take() {
                        if let Some(idx) = item_idx {
                            apply_context_menu_action(self, terminal_runtimes, menu, idx);
                        } else {
                            leave_modal(self);
                        }
                    }
                    return None;
                }

                if self.on_sidebar_divider(mouse.column, mouse.row) {
                    self.drag = Some(DragState {
                        target: DragTarget::SidebarDivider,
                    });
                    self.set_manual_sidebar_width(mouse.column);
                    return None;
                }

                if self.on_sidebar_section_divider(mouse.column, mouse.row) {
                    self.drag = Some(DragState {
                        target: DragTarget::SidebarSectionDivider,
                    });
                    self.set_sidebar_section_split(mouse.row);
                    return None;
                }

                if !in_sidebar {
                    if let Some(border) = self.find_border_at(mouse.column, mouse.row) {
                        self.drag = Some(DragState {
                            target: DragTarget::PaneSplit {
                                path: border.path.clone(),
                                direction: border.direction,
                                area: border.area,
                            },
                        });
                        return None;
                    }

                    if let Some((pane_id, target)) =
                        self.scrollbar_target_at(terminal_runtimes, mouse.column, mouse.row)
                    {
                        self.focus_pane(pane_id);
                        match target {
                            ScrollbarClickTarget::Thumb { grab_row_offset } => {
                                self.drag = Some(DragState {
                                    target: DragTarget::PaneScrollbar {
                                        pane_id,
                                        grab_row_offset,
                                    },
                                });
                            }
                            ScrollbarClickTarget::Track { offset_from_bottom } => {
                                self.set_pane_scroll_offset(
                                    terminal_runtimes,
                                    pane_id,
                                    offset_from_bottom,
                                );
                            }
                        }
                        if self.mode != Mode::Terminal {
                            self.mode = Mode::Terminal;
                        }
                        return None;
                    }
                }

                if self.on_tab_scroll_left_button(mouse.column, mouse.row) {
                    self.scroll_tabs_left();
                    return None;
                }
                if self.on_tab_scroll_right_button(mouse.column, mouse.row) {
                    self.scroll_tabs_right();
                    return None;
                }
                if let (Some(ws_idx), Some(tab_idx)) =
                    (self.active, self.tab_at(mouse.column, mouse.row))
                {
                    self.tab_press = Some(TabPressState {
                        ws_idx,
                        tab_idx,
                        start_col: mouse.column,
                        start_row: mouse.row,
                    });
                    return None;
                }
                if self.on_new_tab_button(mouse.column, mouse.row) {
                    open_new_tab_dialog(self);
                    return None;
                }

                if in_sidebar {
                    if self.sidebar_collapsed {
                        if self.on_collapsed_sidebar_toggle(mouse.column, mouse.row) {
                            self.sidebar_collapsed = false;
                            return None;
                        }

                        if let Some(idx) = self.collapsed_workspace_at_row(mouse.row) {
                            self.switch_workspace(idx);
                            self.mode = Mode::Terminal;
                            return None;
                        }

                        if let Some((ws_idx, _tab_idx, pane_id)) =
                            self.collapsed_agent_detail_target_at(mouse.row)
                        {
                            self.focus_pane_in_workspace(ws_idx, pane_id);
                            self.mode = Mode::Terminal;
                        }
                        return None;
                    }

                    let new_button = self.sidebar_new_button_rect();
                    let on_new_button = mouse.row >= new_button.y
                        && mouse.row < new_button.y + new_button.height
                        && mouse.column >= new_button.x
                        && mouse.column < new_button.x + new_button.width;
                    if on_new_button {
                        self.request_new_workspace = true;
                        return None;
                    }

                    let kanban_button = self.sidebar_kanban_button_rect();
                    let on_kanban_button = kanban_button.width > 0
                        && mouse.row >= kanban_button.y
                        && mouse.row < kanban_button.y + kanban_button.height
                        && mouse.column >= kanban_button.x
                        && mouse.column < kanban_button.x + kanban_button.width;
                    if on_kanban_button {
                        if self.mode == Mode::Kanban {
                            leave_modal(self);
                        } else {
                            self.mode = Mode::Kanban;
                        }
                        return None;
                    }

                    if let Some(target) =
                        self.workspace_list_scrollbar_target_at(mouse.column, mouse.row)
                    {
                        match target {
                            ScrollbarClickTarget::Thumb { grab_row_offset } => {
                                self.drag = Some(DragState {
                                    target: DragTarget::WorkspaceListScrollbar { grab_row_offset },
                                });
                            }
                            ScrollbarClickTarget::Track { offset_from_bottom } => {
                                self.set_workspace_list_offset_from_bottom(offset_from_bottom);
                            }
                        }
                        return None;
                    }

                    let cards = if self.view.workspace_card_areas.is_empty() {
                        crate::ui::compute_workspace_card_areas(self, self.view.sidebar_rect)
                    } else {
                        self.view.workspace_card_areas.clone()
                    };
                    if let Some(card) = cards.iter().find(|card| {
                        mouse.row == card.rect.y
                            && mouse.column == card.rect.x
                            && mouse.column < card.rect.x + card.rect.width
                    }) {
                        if let Some((key, collapsed)) =
                            crate::ui::workspace_parent_group_state(self, card.ws_idx)
                        {
                            if collapsed {
                                self.collapsed_space_keys.remove(&key);
                            } else {
                                self.collapsed_space_keys.insert(key);
                            }
                            self.mark_session_dirty();
                            return None;
                        }
                    }

                    if let Some(idx) = self.workspace_at_row(mouse.row) {
                        self.workspace_press = Some(WorkspacePressState {
                            ws_idx: idx,
                            start_col: mouse.column,
                            start_row: mouse.row,
                        });
                        return None;
                    }

                    if self.on_agent_panel_scope_toggle(mouse.column, mouse.row) {
                        self.agent_panel_scope = match self.agent_panel_scope {
                            AgentPanelScope::CurrentWorkspace => AgentPanelScope::AllWorkspaces,
                            AgentPanelScope::AllWorkspaces => AgentPanelScope::CurrentWorkspace,
                        };
                        self.agent_panel_scroll = 0;
                        self.mark_session_dirty();
                        return None;
                    }

                    if let Some(target) =
                        self.agent_panel_scrollbar_target_at(mouse.column, mouse.row)
                    {
                        match target {
                            ScrollbarClickTarget::Thumb { grab_row_offset } => {
                                self.drag = Some(DragState {
                                    target: DragTarget::AgentPanelScrollbar { grab_row_offset },
                                });
                            }
                            ScrollbarClickTarget::Track { offset_from_bottom } => {
                                self.set_agent_panel_offset_from_bottom(offset_from_bottom);
                            }
                        }
                        return None;
                    }

                    if let Some((ws_idx, _tab_idx, pane_id)) =
                        self.agent_detail_target_at(mouse.row)
                    {
                        self.focus_pane_in_workspace(ws_idx, pane_id);
                        self.mode = Mode::Terminal;
                        return None;
                    }
                } else if let Some(info) = self.pane_at(mouse.column, mouse.row).cloned() {
                    self.focus_pane(info.id);
                    if self.mode != Mode::Terminal {
                        self.mode = Mode::Terminal;
                    }

                    if self.forward_pane_mouse_button(terminal_runtimes, &info, mouse) {
                        self.selection = None;
                        self.selection_autoscroll = None;
                        return None;
                    }

                    let (row, col) = (
                        mouse.row - info.inner_rect.y,
                        mouse.column - info.inner_rect.x,
                    );
                    self.selection = Some(Selection::anchor(
                        info.id,
                        row,
                        col,
                        self.pane_scroll_metrics(terminal_runtimes, info.id),
                    ));
                } else if let Some(info) = self.view.pane_infos.iter().find(|p| {
                    mouse.column >= p.rect.x
                        && mouse.column < p.rect.x + p.rect.width
                        && mouse.row >= p.rect.y
                        && mouse.row < p.rect.y + p.rect.height
                }) {
                    let id = info.id;
                    self.focus_pane(id);
                    if self.mode != Mode::Terminal {
                        self.mode = Mode::Terminal;
                    }
                }
            }

            MouseEventKind::Drag(MouseButton::Left) => {
                if self.selection.is_some() {
                    self.update_selection_drag(terminal_runtimes, mouse.column, mouse.row);
                    return None;
                }

                if self.drag.is_none() {
                    if let Some(info) = self.pane_mouse_target(mouse.column, mouse.row).cloned() {
                        if self.forward_pane_mouse_button(terminal_runtimes, &info, mouse) {
                            self.selection = None;
                            self.selection_autoscroll = None;
                            return None;
                        }
                    }
                }

                let workspace_drop_index = self.workspace_drop_index_at_row(mouse.row);
                let tab_drop_index = self.tab_drop_index_at(mouse.column, mouse.row);
                if self.drag.is_none() {
                    if let Some(press) = &self.workspace_press {
                        let delta_col = mouse.column.abs_diff(press.start_col);
                        let delta_row = mouse.row.abs_diff(press.start_row);
                        let can_reorder = self
                            .workspaces
                            .get(press.ws_idx)
                            .is_some_and(|ws| ws.worktree_space().is_none());
                        if can_reorder && delta_col.max(delta_row) >= WORKSPACE_DRAG_THRESHOLD {
                            self.drag = Some(DragState {
                                target: DragTarget::WorkspaceReorder {
                                    source_ws_idx: press.ws_idx,
                                    insert_idx: workspace_drop_index,
                                },
                            });
                        }
                    } else if let Some(press) = &self.tab_press {
                        let delta_col = mouse.column.abs_diff(press.start_col);
                        let delta_row = mouse.row.abs_diff(press.start_row);
                        if delta_col.max(delta_row) >= TAB_DRAG_THRESHOLD {
                            self.drag = Some(DragState {
                                target: DragTarget::TabReorder {
                                    ws_idx: press.ws_idx,
                                    source_tab_idx: press.tab_idx,
                                    insert_idx: tab_drop_index,
                                },
                            });
                        }
                    }
                }

                if let Some(DragState {
                    target: DragTarget::WorkspaceReorder { insert_idx, .. },
                }) = &mut self.drag
                {
                    *insert_idx = workspace_drop_index;
                } else if let Some(DragState {
                    target:
                        DragTarget::TabReorder {
                            ws_idx, insert_idx, ..
                        },
                }) = &mut self.drag
                {
                    if self.active == Some(*ws_idx) {
                        *insert_idx = tab_drop_index;
                    }
                } else if let Some(drag) = &self.drag {
                    match &drag.target {
                        DragTarget::WorkspaceReorder { .. } | DragTarget::TabReorder { .. } => {}
                        DragTarget::WorkspaceListScrollbar { grab_row_offset } => {
                            if let Some(offset_from_bottom) =
                                self.workspace_list_offset_for_drag_row(mouse.row, *grab_row_offset)
                            {
                                self.set_workspace_list_offset_from_bottom(offset_from_bottom);
                            }
                        }
                        DragTarget::AgentPanelScrollbar { grab_row_offset } => {
                            if let Some(offset_from_bottom) =
                                self.agent_panel_offset_for_drag_row(mouse.row, *grab_row_offset)
                            {
                                self.set_agent_panel_offset_from_bottom(offset_from_bottom);
                            }
                        }
                        DragTarget::PaneSplit {
                            path,
                            direction,
                            area,
                        } => {
                            let ratio = match direction {
                                Direction::Horizontal => {
                                    (mouse.column.saturating_sub(area.x)) as f32
                                        / area.width.max(1) as f32
                                }
                                Direction::Vertical => {
                                    (mouse.row.saturating_sub(area.y)) as f32
                                        / area.height.max(1) as f32
                                }
                            };
                            let ratio = ratio.clamp(0.1, 0.9);
                            let path = path.clone();
                            if let Some(ws) = self.active.and_then(|i| self.workspaces.get_mut(i)) {
                                ws.layout.set_ratio_at(&path, ratio);
                                self.mark_session_dirty();
                            }
                        }
                        DragTarget::PaneScrollbar {
                            pane_id,
                            grab_row_offset,
                        } => {
                            if let Some(offset_from_bottom) = self.scrollbar_offset_for_pane_row(
                                terminal_runtimes,
                                *pane_id,
                                mouse.row,
                                *grab_row_offset,
                            ) {
                                self.set_pane_scroll_offset(
                                    terminal_runtimes,
                                    *pane_id,
                                    offset_from_bottom,
                                );
                            }
                        }
                        DragTarget::SidebarDivider => {
                            self.set_manual_sidebar_width(mouse.column);
                        }
                        DragTarget::SidebarSectionDivider => {
                            self.set_sidebar_section_split(mouse.row);
                        }
                        DragTarget::ReleaseNotesScrollbar { .. }
                        | DragTarget::ProductAnnouncementScrollbar { .. }
                        | DragTarget::KeybindHelpScrollbar { .. } => {}
                    }
                }
            }

            MouseEventKind::Up(MouseButton::Left) => {
                // Mouse-up either finishes a drag selection or releases after a
                // double-click copy; the latter is already copied.
                if let Some(selection) = self.selection.as_ref() {
                    let was_click = selection.was_just_click();
                    let was_already_copied = selection.is_done();

                    self.workspace_press = None;
                    self.tab_press = None;
                    self.drag = None;
                    self.selection_autoscroll = None;
                    if was_click {
                        self.selection = None;
                    } else if !was_already_copied {
                        self.copy_selection(terminal_runtimes);
                    }
                    return None;
                }

                if self.drag.is_none() {
                    if let Some(info) = self.pane_mouse_target(mouse.column, mouse.row).cloned() {
                        if self.forward_pane_mouse_button(terminal_runtimes, &info, mouse) {
                            self.selection = None;
                            self.selection_autoscroll = None;
                            self.workspace_press = None;
                            self.tab_press = None;
                            self.drag = None;
                            return None;
                        }
                    }
                }

                let workspace_press = self.workspace_press.take();
                let tab_press = self.tab_press.take();
                match self.drag.take() {
                    Some(DragState {
                        target:
                            DragTarget::WorkspaceReorder {
                                source_ws_idx,
                                insert_idx: Some(insert_idx),
                            },
                    }) => {
                        self.move_workspace(source_ws_idx, insert_idx);
                    }
                    Some(DragState {
                        target:
                            DragTarget::TabReorder {
                                ws_idx,
                                source_tab_idx,
                                insert_idx: Some(insert_idx),
                            },
                    }) => {
                        if self.active == Some(ws_idx) {
                            self.move_tab(source_tab_idx, insert_idx);
                            self.mode = Mode::Terminal;
                        }
                    }
                    Some(_) => {}
                    None => {
                        if let Some(press) = workspace_press {
                            self.switch_workspace(press.ws_idx);
                            self.mode = Mode::Terminal;
                            return None;
                        }
                        if let Some(press) = tab_press {
                            if self.active == Some(press.ws_idx) {
                                self.switch_tab(press.tab_idx);
                                self.mode = Mode::Terminal;
                                return None;
                            }
                        }
                    }
                }
            }

            MouseEventKind::Up(MouseButton::Middle) | MouseEventKind::Drag(MouseButton::Middle)
                if !in_sidebar =>
            {
                if let Some(info) = self.pane_mouse_target(mouse.column, mouse.row).cloned() {
                    let _ = self.forward_pane_mouse_button(terminal_runtimes, &info, mouse);
                }
            }

            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                if self.on_tab_bar(mouse.column, mouse.row) =>
            {
                match mouse.kind {
                    MouseEventKind::ScrollUp => self.previous_tab(),
                    MouseEventKind::ScrollDown => self.next_tab(),
                    _ => {}
                }
            }

            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                if !in_sidebar && self.scroll_selection_with_wheel(terminal_runtimes, mouse) => {}

            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown if !in_sidebar => {
                self.selection = None;
                self.selection_autoscroll = None;
                self.handle_terminal_wheel(terminal_runtimes, mouse);
            }

            MouseEventKind::ScrollUp if in_sidebar => {
                let agent_area = self.agent_panel_rect();
                let over_agent_panel = agent_area != Rect::default()
                    && mouse.row >= agent_area.y
                    && mouse.row < agent_area.y + agent_area.height;
                if over_agent_panel {
                    if crate::ui::should_show_scrollbar(crate::ui::agent_panel_scroll_metrics(
                        self, agent_area,
                    )) {
                        self.scroll_agent_panel(-1);
                    }
                } else if crate::ui::should_show_scrollbar(
                    crate::ui::workspace_list_scroll_metrics(self, self.workspace_list_rect()),
                ) {
                    self.scroll_workspace_list(-1);
                } else {
                    self.move_selected_workspace_by_visible_delta(-1);
                }
            }
            MouseEventKind::ScrollDown if in_sidebar => {
                let agent_area = self.agent_panel_rect();
                let over_agent_panel = agent_area != Rect::default()
                    && mouse.row >= agent_area.y
                    && mouse.row < agent_area.y + agent_area.height;
                if over_agent_panel {
                    if crate::ui::should_show_scrollbar(crate::ui::agent_panel_scroll_metrics(
                        self, agent_area,
                    )) {
                        self.scroll_agent_panel(1);
                    }
                } else if crate::ui::should_show_scrollbar(
                    crate::ui::workspace_list_scroll_metrics(self, self.workspace_list_rect()),
                ) {
                    self.scroll_workspace_list(1);
                } else {
                    self.move_selected_workspace_by_visible_delta(1);
                }
            }

            MouseEventKind::Moved if self.mode == Mode::ContextMenu => {
                let hovered = self.context_menu_item_at(mouse.column, mouse.row);
                if let Some(menu) = &mut self.context_menu {
                    menu.list.hover(hovered);
                }
            }

            MouseEventKind::Down(MouseButton::Right) if in_sidebar && !self.sidebar_collapsed => {
                self.workspace_press = None;
                self.tab_press = None;
                if self
                    .workspace_list_scrollbar_target_at(mouse.column, mouse.row)
                    .is_some()
                {
                    return None;
                }
                if let Some(idx) = self.workspace_at_row(mouse.row) {
                    self.selected = idx;
                    let kind = self
                        .workspaces
                        .get(idx)
                        .and_then(|ws| {
                            let group_state = crate::ui::workspace_parent_group_state(self, idx);
                            let git_space = ws.git_space().cloned().or_else(|| {
                                ws.resolved_identity_cwd_from(&self.terminals, terminal_runtimes)
                                    .as_deref()
                                    .and_then(crate::workspace::git_space_metadata)
                            });
                            let is_linked_worktree = ws.worktree_space().map_or_else(
                                || {
                                    git_space
                                        .as_ref()
                                        .is_some_and(|space| space.is_linked_worktree)
                                },
                                |space| space.is_linked_worktree,
                            );
                            let show_git_menu = ws.worktree_space().is_some()
                                || git_space
                                    .as_ref()
                                    .is_some_and(|space| !space.is_linked_worktree);
                            show_git_menu.then_some(ContextMenuKind::GitWorkspace {
                                ws_idx: idx,
                                is_linked_worktree,
                                has_worktree_children: group_state.is_some(),
                                collapsed: group_state
                                    .as_ref()
                                    .is_some_and(|(_, collapsed)| *collapsed),
                            })
                        })
                        .unwrap_or(ContextMenuKind::Workspace { ws_idx: idx });
                    self.context_menu = Some(ContextMenuState {
                        kind,
                        x: mouse.column,
                        y: mouse.row,
                        list: MenuListState::new(0),
                    });
                    self.mode = Mode::ContextMenu;
                }
            }

            MouseEventKind::Down(MouseButton::Right)
                if self.tab_at(mouse.column, mouse.row).is_some() =>
            {
                if let (Some(ws_idx), Some(tab_idx)) =
                    (self.active, self.tab_at(mouse.column, mouse.row))
                {
                    self.switch_tab(tab_idx);
                    self.context_menu = Some(ContextMenuState {
                        kind: ContextMenuKind::Tab { ws_idx, tab_idx },
                        x: mouse.column,
                        y: mouse.row,
                        list: MenuListState::new(0),
                    });
                    self.mode = Mode::ContextMenu;
                }
            }

            MouseEventKind::Down(MouseButton::Right) if !in_sidebar => {
                if let Some(info) = self.pane_mouse_target(mouse.column, mouse.row).cloned() {
                    self.focus_pane(info.id);
                    let has_manual_label = self
                        .active
                        .and_then(|ws_idx| self.workspaces.get(ws_idx))
                        .and_then(|ws| ws.pane_state(info.id))
                        .and_then(|pane| self.terminals.get(&pane.attached_terminal_id))
                        .and_then(|terminal| terminal.manual_label.as_ref())
                        .is_some();
                    self.context_menu = Some(ContextMenuState {
                        kind: ContextMenuKind::Pane {
                            pane_id: info.id,
                            has_manual_label,
                        },
                        x: mouse.column,
                        y: mouse.row,
                        list: MenuListState::new(0),
                    });
                    self.mode = Mode::ContextMenu;
                }
            }

            _ => {}
        }

        None
    }

    fn handle_mobile_mouse(&mut self, mouse: MouseEvent) -> bool {
        if self.mode == Mode::Navigate {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.scroll_mobile_switcher_at(mouse.column, mouse.row, -1);
                    return true;
                }
                MouseEventKind::ScrollDown => {
                    self.scroll_mobile_switcher_at(mouse.column, mouse.row, 1);
                    return true;
                }
                MouseEventKind::Down(MouseButton::Left) => {}
                _ => return true,
            }
        } else if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
            return false;
        }

        if self.mode != Mode::Navigate {
            if !matches!(self.mode, Mode::Terminal | Mode::Resize | Mode::Kanban) {
                return false;
            }
            if rect_contains(self.view.mobile_menu_hit_area, mouse.column, mouse.row) {
                self.mobile_switcher_scroll = 0;
                self.mode = Mode::Navigate;
                return true;
            }
            return false;
        }

        let areas = crate::ui::mobile_switcher_areas(self);
        if rect_contains(areas.close, mouse.column, mouse.row) {
            self.mode = Mode::Terminal;
            return true;
        }

        match crate::ui::mobile_switcher_target_at(self, mouse.column, mouse.row) {
            Some(crate::ui::MobileSwitcherTarget::NewWorkspace) => {
                self.request_new_workspace = true;
            }
            Some(crate::ui::MobileSwitcherTarget::Workspace(ws_idx)) => {
                self.switch_workspace(ws_idx);
                self.mode = Mode::Terminal;
            }
            Some(crate::ui::MobileSwitcherTarget::NewTab) => {
                open_new_tab_dialog(self);
            }
            Some(crate::ui::MobileSwitcherTarget::Tab(tab_idx)) => {
                self.switch_tab(tab_idx);
                self.mode = Mode::Terminal;
            }
            Some(crate::ui::MobileSwitcherTarget::Kanban) => {
                self.mode = Mode::Kanban;
            }
            Some(crate::ui::MobileSwitcherTarget::Agent {
                ws_idx,
                tab_idx: _,
                pane_id,
            }) => {
                self.focus_pane_in_workspace(ws_idx, pane_id);
                self.mode = Mode::Terminal;
            }
            Some(crate::ui::MobileSwitcherTarget::Menu(action_idx)) => {
                let actions = global_menu_actions(self);
                if let Some(action) = actions.get(action_idx).copied() {
                    apply_global_menu_action(self, action);
                }
            }
            None => {}
        }

        true
    }

    fn scroll_mobile_switcher_at(&mut self, _col: u16, _row: u16, delta: i16) {
        let max_scroll = crate::ui::mobile_switcher_max_scroll(self);
        apply_scroll(
            &mut self.mobile_switcher_scroll,
            delta.saturating_mul(2),
            max_scroll,
        );
    }

    pub(super) fn screen_rect(&self) -> Rect {
        let sidebar = self.view.sidebar_rect;
        let terminal = self.view.terminal_area;
        let x = sidebar.x.min(terminal.x);
        let y = sidebar.y.min(terminal.y);
        let right = (sidebar.x + sidebar.width).max(terminal.x + terminal.width);
        let bottom = (sidebar.y + sidebar.height).max(terminal.y + terminal.height);
        Rect::new(x, y, right.saturating_sub(x), bottom.saturating_sub(y))
    }

    pub(crate) fn context_menu_rect(&self) -> Option<Rect> {
        let menu = self.context_menu.as_ref()?;
        let screen = self.screen_rect();
        let max_item_w = menu
            .items()
            .iter()
            .map(|item| item.len() as u16)
            .max()
            .unwrap_or(0);
        let menu_w = (max_item_w + 4).max(14).min(screen.width.max(1));
        let menu_h = (menu.items().len() as u16 + 2).min(screen.height.max(1));
        let x = menu.x.min(screen.x + screen.width.saturating_sub(menu_w));
        let y = menu.y.min(screen.y + screen.height.saturating_sub(menu_h));
        Some(Rect::new(x, y, menu_w, menu_h))
    }

    pub(crate) fn confirm_close_rect(&self) -> Rect {
        crate::ui::confirm_close_popup_rect(self.view.terminal_area).unwrap_or_default()
    }

    fn context_menu_item_at(&self, col: u16, row: u16) -> Option<usize> {
        let menu_rect = self.context_menu_rect()?;
        let inner_x = menu_rect.x + 1;
        let inner_y = menu_rect.y + 1;
        let inner_w = menu_rect.width.saturating_sub(2);
        let inner_h = menu_rect.height.saturating_sub(2);
        let item_count = self
            .context_menu
            .as_ref()
            .map(|menu| menu.items().len() as u16)
            .unwrap_or(0);
        if col >= inner_x
            && col < inner_x + inner_w
            && row >= inner_y
            && row < inner_y + inner_h.min(item_count)
        {
            Some((row - inner_y) as usize)
        } else {
            None
        }
    }

    pub(super) fn tab_at(&self, col: u16, row: u16) -> Option<usize> {
        self.view
            .tab_hit_areas
            .iter()
            .enumerate()
            .find_map(|(idx, area)| {
                (area.width > 0
                    && row >= area.y
                    && row < area.y + area.height
                    && col >= area.x
                    && col < area.x + area.width)
                    .then_some(idx)
            })
    }

    pub(super) fn on_tab_bar(&self, col: u16, row: u16) -> bool {
        let area = self.view.tab_bar_rect;
        area.width > 0
            && row >= area.y
            && row < area.y + area.height
            && col >= area.x
            && col < area.x + area.width
    }

    pub(super) fn on_tab_scroll_left_button(&self, col: u16, row: u16) -> bool {
        let area = self.view.tab_scroll_left_hit_area;
        area.width > 0
            && row >= area.y
            && row < area.y + area.height
            && col >= area.x
            && col < area.x + area.width
    }

    pub(super) fn on_tab_scroll_right_button(&self, col: u16, row: u16) -> bool {
        let area = self.view.tab_scroll_right_hit_area;
        area.width > 0
            && row >= area.y
            && row < area.y + area.height
            && col >= area.x
            && col < area.x + area.width
    }

    pub(super) fn tab_drop_index_at(&self, col: u16, row: u16) -> Option<usize> {
        if !self.on_tab_bar(col, row) {
            return None;
        }

        let visible_tabs: Vec<_> = self
            .view
            .tab_hit_areas
            .iter()
            .enumerate()
            .filter(|(_, rect)| rect.width > 0)
            .collect();
        let (first_idx, first_rect) = *visible_tabs.first()?;
        let (last_idx, last_rect) = *visible_tabs.last()?;

        if self.on_tab_scroll_left_button(col, row) {
            return Some(0);
        }
        if self.on_tab_scroll_right_button(col, row) {
            return self
                .active
                .and_then(|idx| self.workspaces.get(idx))
                .map(|ws| ws.tabs.len());
        }

        let left_edge = if first_idx == 0 {
            first_rect.x
        } else {
            self.view.tab_scroll_left_hit_area.x + self.view.tab_scroll_left_hit_area.width
        };
        let right_edge = if self
            .active
            .and_then(|idx| self.workspaces.get(idx))
            .is_some_and(|ws| last_idx + 1 >= ws.tabs.len())
        {
            last_rect.x + last_rect.width
        } else {
            self.view.tab_scroll_right_hit_area.x.saturating_sub(1)
        };

        if col <= left_edge {
            return Some(first_idx);
        }
        if col >= right_edge {
            return Some(last_idx + 1);
        }

        for (idx, rect) in visible_tabs {
            let midpoint = rect.x + rect.width / 2;
            if col < midpoint {
                return Some(idx);
            }
            if col < rect.x + rect.width {
                return Some(idx + 1);
            }
        }

        Some(last_idx + 1)
    }

    pub(super) fn on_new_tab_button(&self, col: u16, row: u16) -> bool {
        let area = self.view.new_tab_hit_area;
        area.width > 0
            && row >= area.y
            && row < area.y + area.height
            && col >= area.x
            && col < area.x + area.width
    }

    pub(super) fn find_border_at(&self, col: u16, row: u16) -> Option<&SplitBorder> {
        self.view.split_borders.iter().find(|b| match b.direction {
            Direction::Horizontal => {
                col >= b.pos.saturating_sub(1)
                    && col <= b.pos
                    && row >= b.area.y
                    && row < b.area.y + b.area.height
            }
            Direction::Vertical => {
                row >= b.pos.saturating_sub(1)
                    && row <= b.pos
                    && col >= b.area.x
                    && col < b.area.x + b.area.width
            }
        })
    }

    pub(super) fn pane_at(&self, col: u16, row: u16) -> Option<&PaneInfo> {
        self.view.pane_infos.iter().find(|p| {
            col >= p.inner_rect.x
                && col < p.inner_rect.x + p.inner_rect.width
                && row >= p.inner_rect.y
                && row < p.inner_rect.y + p.inner_rect.height
        })
    }

    pub(super) fn pane_mouse_target(&self, col: u16, row: u16) -> Option<&PaneInfo> {
        self.pane_at(col, row)
            .or_else(|| self.pane_frame_at(col, row))
    }

    pub(crate) fn pane_info_by_id(&self, pane_id: crate::layout::PaneId) -> Option<&PaneInfo> {
        self.view.pane_infos.iter().find(|info| info.id == pane_id)
    }

    pub(super) fn pane_frame_at(&self, col: u16, row: u16) -> Option<&PaneInfo> {
        self.view.pane_infos.iter().find(|p| {
            col >= p.rect.x
                && col < p.rect.x + p.rect.width
                && row >= p.rect.y
                && row < p.rect.y + p.rect.height
        })
    }

    pub(super) fn focus_pane(&mut self, pane_id: crate::layout::PaneId) {
        if let Some(ws_idx) = self.active {
            self.focus_pane_in_workspace(ws_idx, pane_id);
        }
    }

    fn clickable_toast_at(&self, col: u16, row: u16) -> bool {
        self.toast
            .as_ref()
            .is_some_and(|toast| toast.target.is_some())
            && rect_contains(self.view.toast_hit_area, col, row)
    }

    pub(crate) fn focus_toast_target(&mut self) {
        let Some(target) = self.toast.as_ref().and_then(|toast| toast.target.clone()) else {
            return;
        };
        let Some(ws_idx) = self
            .workspaces
            .iter()
            .position(|workspace| workspace.id == target.workspace_id)
        else {
            return;
        };
        let Some(_tab_idx) = self.workspaces[ws_idx].find_tab_index_for_pane(target.pane_id) else {
            return;
        };

        self.focus_pane_in_workspace(ws_idx, target.pane_id);
        self.toast = None;
        self.mode = Mode::Terminal;
    }

    pub(crate) fn scroll_pane_up(
        &self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        pane_id: crate::layout::PaneId,
        lines: usize,
    ) {
        if let Some(ws_idx) = self.active {
            if let Some(rt) = self.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, pane_id)
            {
                rt.scroll_up(lines);
            }
        }
    }

    pub(crate) fn scroll_pane_down(
        &self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        pane_id: crate::layout::PaneId,
        lines: usize,
    ) {
        if let Some(ws_idx) = self.active {
            if let Some(rt) = self.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, pane_id)
            {
                rt.scroll_down(lines);
            }
        }
    }

    pub(crate) fn pane_scroll_metrics(
        &self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        pane_id: crate::layout::PaneId,
    ) -> Option<crate::pane::ScrollMetrics> {
        self.active
            .and_then(|i| self.runtime_for_pane_in_workspace(terminal_runtimes, i, pane_id))
            .and_then(crate::terminal::TerminalRuntime::scroll_metrics)
    }

    pub(super) fn handle_terminal_wheel(
        &mut self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        mouse: MouseEvent,
    ) {
        let lines_per_notch = self.mouse_scroll_lines;

        if let Some(info) = self.pane_at(mouse.column, mouse.row).cloned() {
            self.focus_pane(info.id);
            if self.forward_pane_wheel(terminal_runtimes, &info, mouse) {
                return;
            }
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.scroll_pane_up(terminal_runtimes, info.id, lines_per_notch)
                }
                MouseEventKind::ScrollDown => {
                    self.scroll_pane_down(terminal_runtimes, info.id, lines_per_notch)
                }
                _ => {}
            }
            return;
        }

        if let Some(info) = self.pane_frame_at(mouse.column, mouse.row).cloned() {
            self.focus_pane(info.id);
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.scroll_pane_up(terminal_runtimes, info.id, lines_per_notch)
                }
                MouseEventKind::ScrollDown => {
                    self.scroll_pane_down(terminal_runtimes, info.id, lines_per_notch)
                }
                _ => {}
            }
            return;
        }

        if let Some(ws_idx) = self.active {
            if let Some(rt) = self.focused_runtime_in_workspace(terminal_runtimes, ws_idx) {
                match mouse.kind {
                    MouseEventKind::ScrollUp => rt.scroll_up(lines_per_notch),
                    MouseEventKind::ScrollDown => rt.scroll_down(lines_per_notch),
                    _ => {}
                }
            }
        }
    }

    pub(super) fn forward_pane_mouse_button(
        &self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        info: &PaneInfo,
        mouse: MouseEvent,
    ) -> bool {
        let Some(ws_idx) = self.active else {
            return false;
        };
        let Some(rt) = self.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, info.id)
        else {
            return false;
        };
        let column = mouse.column.saturating_sub(info.inner_rect.x);
        let row = mouse.row.saturating_sub(info.inner_rect.y);
        let Some(bytes) = rt.encode_mouse_button(mouse.kind, column, row, mouse.modifiers) else {
            return false;
        };
        rt.scroll_reset();
        if let Err(err) = rt.try_send_bytes(Bytes::from(bytes)) {
            warn!(pane = info.id.raw(), err = %err, kind = ?mouse.kind, "failed to forward mouse button event");
        }
        true
    }

    fn forward_pane_reported_wheel(
        &self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        info: &PaneInfo,
        mouse: MouseEvent,
    ) -> bool {
        let Some(ws_idx) = self.active else {
            return false;
        };
        let Some(rt) = self.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, info.id)
        else {
            return false;
        };
        if !rt
            .input_state()
            .is_some_and(crate::pane::InputState::mouse_reporting_enabled)
        {
            return false;
        }
        rt.scroll_reset();
        let column = mouse.column.saturating_sub(info.inner_rect.x);
        let row = mouse.row.saturating_sub(info.inner_rect.y);
        let Some(bytes) = rt.encode_mouse_wheel(mouse.kind, column, row, mouse.modifiers) else {
            warn!(pane = info.id.raw(), kind = ?mouse.kind, "failed to encode mouse wheel event");
            return true;
        };
        if let Err(err) = rt.try_send_bytes(Bytes::from(bytes)) {
            warn!(pane = info.id.raw(), err = %err, "failed to forward mouse wheel event");
        }
        true
    }

    pub(super) fn forward_pane_wheel(
        &self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        info: &PaneInfo,
        mouse: MouseEvent,
    ) -> bool {
        let Some(ws_idx) = self.active else {
            return false;
        };
        let Some(rt) = self.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, info.id)
        else {
            return false;
        };
        match rt.wheel_routing() {
            Some(crate::pane::WheelRouting::HostScroll) | None => false,
            Some(crate::pane::WheelRouting::MouseReport) => {
                rt.scroll_reset();
                let column = mouse.column.saturating_sub(info.inner_rect.x);
                let row = mouse.row.saturating_sub(info.inner_rect.y);
                let Some(bytes) = rt.encode_mouse_wheel(mouse.kind, column, row, mouse.modifiers)
                else {
                    warn!(pane = info.id.raw(), kind = ?mouse.kind, "failed to encode mouse wheel event");
                    return true;
                };
                if let Err(err) = rt.try_send_bytes(Bytes::from(bytes)) {
                    warn!(pane = info.id.raw(), err = %err, "failed to forward mouse wheel event");
                }
                true
            }
            Some(crate::pane::WheelRouting::AlternateScroll) => {
                rt.scroll_reset();
                let Some(bytes) = rt.encode_alternate_scroll(mouse.kind) else {
                    return true;
                };
                if let Err(err) = rt.try_send_bytes(Bytes::from(bytes)) {
                    warn!(pane = info.id.raw(), err = %err, "failed to forward alternate-scroll key");
                }
                true
            }
        }
    }

    pub(super) fn set_pane_scroll_offset(
        &self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        pane_id: crate::layout::PaneId,
        offset_from_bottom: usize,
    ) {
        if let Some(ws_idx) = self.active {
            if let Some(rt) = self.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, pane_id)
            {
                rt.set_scroll_offset_from_bottom(offset_from_bottom);
            }
        }
    }

    pub(super) fn scrollbar_target_at(
        &self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        col: u16,
        row: u16,
    ) -> Option<(crate::layout::PaneId, ScrollbarClickTarget)> {
        let ws_idx = self.active?;
        let info = self.view.pane_infos.iter().find(|info| {
            crate::ui::pane_scrollbar_rect(info).is_some_and(|track| {
                col >= track.x
                    && col < track.x + track.width
                    && row >= track.y
                    && row < track.y + track.height
            })
        })?;
        let rt = self.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, info.id)?;
        let metrics = rt.scroll_metrics()?;
        if metrics.max_offset_from_bottom == 0 {
            return None;
        }
        let track = crate::ui::pane_scrollbar_rect(info)?;
        if let Some(grab_row_offset) = crate::ui::scrollbar_thumb_grab_offset(metrics, track, row) {
            Some((info.id, ScrollbarClickTarget::Thumb { grab_row_offset }))
        } else {
            Some((
                info.id,
                ScrollbarClickTarget::Track {
                    offset_from_bottom: crate::ui::scrollbar_offset_from_row(metrics, track, row),
                },
            ))
        }
    }

    pub(super) fn scrollbar_offset_for_pane_row(
        &self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        pane_id: crate::layout::PaneId,
        row: u16,
        grab_row_offset: u16,
    ) -> Option<usize> {
        let ws_idx = self.active?;
        let info = self
            .view
            .pane_infos
            .iter()
            .find(|info| info.id == pane_id)?;
        let track = crate::ui::pane_scrollbar_rect(info)?;
        let rt = self.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, pane_id)?;
        let metrics = rt.scroll_metrics()?;
        if metrics.max_offset_from_bottom == 0 {
            return None;
        }
        Some(crate::ui::scrollbar_offset_from_drag_row(
            metrics,
            track,
            row,
            grab_row_offset,
        ))
    }
}

#[cfg(test)]
pub(super) fn wheel_routing(input_state: crate::pane::InputState) -> WheelRouting {
    if input_state.mouse_protocol_mode.reporting_enabled() {
        WheelRouting::MouseReport
    } else if input_state.alternate_screen && input_state.mouse_alternate_scroll {
        WheelRouting::AlternateScroll
    } else {
        WheelRouting::HostScroll
    }
}

fn rect_contains(rect: Rect, col: u16, row: u16) -> bool {
    rect.width > 0
        && rect.height > 0
        && col >= rect.x
        && col < rect.x + rect.width
        && row >= rect.y
        && row < rect.y + rect.height
}

fn apply_scroll(scroll: &mut usize, delta: i16, max_scroll: usize) {
    if delta.is_negative() {
        *scroll = scroll.saturating_sub(delta.unsigned_abs() as usize);
    } else {
        *scroll = scroll.saturating_add(delta as usize).min(max_scroll);
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
    use ratatui::layout::{Direction, Rect};

    use super::super::{
        app_for_mouse_test, capture_snapshot, handle_context_menu_key, mouse, numbered_lines_bytes,
        root_layout_ratio,
    };
    use super::*;
    use crate::{
        app::state::{ContextMenuKind, ContextMenuState, MenuListState, Mode, ViewLayout},
        detect::{Agent, AgentState},
        workspace::Workspace,
    };

    #[tokio::test]
    async fn terminal_wheel_uses_configured_mouse_scroll_lines() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let pane_infos = ws.tabs[0].layout.panes(Rect::new(26, 2, 80, 18));
        let info = pane_infos[0].clone();
        ws.tabs[0].runtimes.insert(
            pane_id,
            crate::terminal::TerminalRuntime::test_with_scrollback_bytes(
                info.inner_rect.width,
                info.inner_rect.height,
                16 * 1024,
                &numbered_lines_bytes(64),
            ),
        );

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.view.pane_infos = pane_infos;
        app.state.mouse_scroll_lines = 7;

        app.handle_mouse(mouse(
            MouseEventKind::ScrollUp,
            info.inner_rect.x + 1,
            info.inner_rect.y + 1,
        ));

        let metrics = app
            .state
            .runtime_for_pane_in_workspace(&app.terminal_runtimes, 0, pane_id)
            .and_then(crate::terminal::TerminalRuntime::scroll_metrics)
            .expect("scroll metrics after wheel");
        assert_eq!(metrics.offset_from_bottom, 7);
    }

    fn sample_worktree_open_state() -> crate::app::state::WorktreeOpenState {
        crate::app::state::WorktreeOpenState {
            source_workspace_id: "source".into(),
            source_existing_membership: None,
            source_checkout_path: "/repo/herdr".into(),
            source_repo_root: "/repo/herdr".into(),
            repo_key: "repo-key".into(),
            repo_name: "herdr".into(),
            entries: vec![
                crate::app::state::WorktreeOpenEntry {
                    path: "/repo/herdr".into(),
                    branch: Some("main".into()),
                    is_linked_worktree: false,
                    already_open_ws_idx: Some(0),
                },
                crate::app::state::WorktreeOpenEntry {
                    path: "/repo/herdr-issue".into(),
                    branch: Some("worktree/issue".into()),
                    is_linked_worktree: true,
                    already_open_ws_idx: None,
                },
            ],
            selected: 0,
            error: None,
        }
    }

    #[test]
    fn hovering_context_menu_updates_highlight() {
        let mut app = app_for_mouse_test();
        app.state.context_menu = Some(ContextMenuState {
            kind: ContextMenuKind::Workspace { ws_idx: 0 },
            x: 2,
            y: 2,
            list: MenuListState::new(0),
        });
        app.state.mode = Mode::ContextMenu;

        let menu = app.state.context_menu_rect().unwrap();
        app.handle_mouse(mouse(MouseEventKind::Moved, menu.x + 2, menu.y + 2));

        assert_eq!(app.state.context_menu.unwrap().list.highlighted, 1);
    }

    #[test]
    fn clicking_agent_toast_focuses_target_pane() {
        let mut app = app_for_mouse_test();
        let active = Workspace::test_new("active");
        let mut background = Workspace::test_new("background");
        let first_pane = background.tabs[0].root_pane;
        let target_pane = background.test_split(Direction::Horizontal);
        background.tabs[0].layout.focus_pane(first_pane);

        app.state.workspaces = vec![active, background];
        app.state.ensure_test_terminals();
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.toast_config.delivery = crate::config::ToastDelivery::Herdr;
        let target_terminal_id = app.state.workspaces[1]
            .panes
            .get(&target_pane)
            .unwrap()
            .attached_terminal_id
            .clone();
        app.state
            .terminals
            .get_mut(&target_terminal_id)
            .unwrap()
            .state = AgentState::Working;

        app.state
            .handle_app_event(crate::events::AppEvent::StateChanged {
                pane_id: target_pane,
                agent: Some(Agent::Pi),
                state: AgentState::Idle,
                visible_blocker: false,
                visible_idle: false,
                visible_working: false,
                process_exited: false,
                observed_at: std::time::Instant::now(),
            });
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let hit = app.state.view.toast_hit_area;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            hit.x + 1,
            hit.y + 1,
        ));

        assert_eq!(app.state.active, Some(1));
        assert_eq!(app.state.workspaces[1].focused_pane_id(), Some(target_pane));
        assert!(app.state.toast.is_none());
        assert_eq!(app.state.mode, Mode::Terminal);

        app.state.last_pane();

        assert_eq!(app.state.active, Some(0));
        assert_eq!(
            app.state.workspaces[0].focused_pane_id(),
            Some(app.state.workspaces[0].tabs[0].root_pane)
        );
    }

    #[test]
    fn toast_click_does_not_steal_mouse_from_settings_overlay() {
        let mut app = app_for_mouse_test();
        let active = Workspace::test_new("active");
        let background = Workspace::test_new("background");
        let target_pane = background.tabs[0].root_pane;
        let workspace_id = background.id.clone();

        app.state.workspaces = vec![active, background];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.toast = Some(crate::app::state::ToastNotification {
            kind: crate::app::state::ToastKind::Finished,
            title: "pi finished".into(),
            context: "background · 2".into(),
            target: Some(crate::app::state::ToastTarget {
                workspace_id,
                pane_id: target_pane,
            }),
        });
        app.state.mode = Mode::Settings;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let hit = app.state.view.toast_hit_area;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            hit.x + 1,
            hit.y + 1,
        ));

        assert_eq!(app.state.active, Some(0));
        assert!(app.state.toast.is_some());
    }

    #[test]
    fn clicking_confirm_close_accepts_workspace_close() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("a"), Workspace::test_new("b")];
        app.state.active = Some(0);
        app.state.selected = 1;
        app.state.mode = Mode::ConfirmClose;

        let popup = app.state.confirm_close_rect();
        let inner = Rect::new(
            popup.x + 1,
            popup.y + 1,
            popup.width.saturating_sub(2),
            popup.height.saturating_sub(2),
        );
        let (confirm, _) = crate::ui::confirm_close_button_rects(inner);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            confirm.x,
            confirm.y,
        ));

        assert_eq!(app.state.workspaces.len(), 1);
        assert_eq!(app.state.mode, Mode::Terminal);
    }

    #[test]
    fn clicking_open_worktree_row_selects_and_requests_open() {
        let mut app = app_for_mouse_test();
        app.state.mode = Mode::OpenExistingWorktree;
        app.state.worktree_open = Some(sample_worktree_open_state());
        let inner =
            crate::ui::open_existing_worktree_inner_rect(app.state.screen_rect(), 2).unwrap();

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            inner.x + 1,
            inner.y + 3,
        ));

        assert_eq!(app.state.worktree_open.as_ref().unwrap().selected, 1);
        assert!(app.state.request_submit_worktree_open);
    }

    #[test]
    fn clicking_open_worktree_buttons_requests_open_or_cancels() {
        let mut app = app_for_mouse_test();
        app.state.mode = Mode::OpenExistingWorktree;
        app.state.worktree_open = Some(sample_worktree_open_state());
        let inner =
            crate::ui::open_existing_worktree_inner_rect(app.state.screen_rect(), 2).unwrap();
        let (open, _) = crate::ui::open_existing_worktree_button_rects(inner);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            open.x,
            open.y,
        ));

        assert!(app.state.worktree_open.is_some());
        assert!(app.state.request_submit_worktree_open);

        let mut app = app_for_mouse_test();
        app.state.mode = Mode::OpenExistingWorktree;
        app.state.worktree_open = Some(sample_worktree_open_state());
        let inner =
            crate::ui::open_existing_worktree_inner_rect(app.state.screen_rect(), 2).unwrap();
        let (_, cancel) = crate::ui::open_existing_worktree_button_rects(inner);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            cancel.x,
            cancel.y,
        ));

        assert!(app.state.worktree_open.is_none());
        assert_eq!(app.state.mode, Mode::Navigate);
    }

    #[test]
    fn scrolling_open_worktree_picker_moves_selection() {
        let mut app = app_for_mouse_test();
        app.state.mode = Mode::OpenExistingWorktree;
        app.state.worktree_open = Some(sample_worktree_open_state());

        app.handle_mouse(mouse(MouseEventKind::ScrollDown, 1, 1));
        assert_eq!(app.state.worktree_open.as_ref().unwrap().selected, 1);

        app.handle_mouse(mouse(MouseEventKind::ScrollUp, 1, 1));
        assert_eq!(app.state.worktree_open.as_ref().unwrap().selected, 0);
    }

    #[test]
    fn clicking_remove_worktree_buttons_requests_remove_or_cancels() {
        let mut app = app_for_mouse_test();
        app.state.mode = Mode::ConfirmRemoveWorktree;
        app.state.worktree_remove = Some(crate::app::state::WorktreeRemoveState {
            workspace_id: "issue".into(),
            repo_root: "/repo/herdr".into(),
            path: "/repo/herdr-issue".into(),
            error: None,
            removing: false,
            force_confirmation: false,
        });
        let popup = crate::ui::remove_worktree_popup_rect(app.state.screen_rect()).unwrap();
        let inner = Rect::new(
            popup.x + 1,
            popup.y + 1,
            popup.width.saturating_sub(2),
            popup.height.saturating_sub(2),
        );
        let (remove, _) = crate::ui::remove_worktree_button_rects(inner, false);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            remove.x,
            remove.y,
        ));

        assert!(app.state.worktree_remove.is_some());
        assert!(app.state.request_submit_worktree_remove);

        let mut app = app_for_mouse_test();
        app.state.mode = Mode::ConfirmRemoveWorktree;
        app.state.worktree_remove = Some(crate::app::state::WorktreeRemoveState {
            workspace_id: "issue".into(),
            repo_root: "/repo/herdr".into(),
            path: "/repo/herdr-issue".into(),
            error: None,
            removing: false,
            force_confirmation: false,
        });
        let popup = crate::ui::remove_worktree_popup_rect(app.state.screen_rect()).unwrap();
        let inner = Rect::new(
            popup.x + 1,
            popup.y + 1,
            popup.width.saturating_sub(2),
            popup.height.saturating_sub(2),
        );
        let (_, cancel) = crate::ui::remove_worktree_button_rects(inner, false);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            cancel.x,
            cancel.y,
        ));

        assert!(app.state.worktree_remove.is_none());
        assert_eq!(app.state.mode, Mode::Navigate);
    }

    #[test]
    fn clicking_confirm_close_accepts_after_workspace_context_menu_close() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("a"), Workspace::test_new("b")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        app.state.context_menu = Some(ContextMenuState {
            kind: ContextMenuKind::Workspace { ws_idx: 1 },
            x: 2,
            y: 2,
            list: MenuListState::new(1),
        });
        app.state.mode = Mode::ContextMenu;
        handle_context_menu_key(
            &mut app.state,
            &mut app.terminal_runtimes,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );
        assert_eq!(app.state.mode, Mode::ConfirmClose);
        assert_eq!(app.state.selected, 1);

        let popup = app.state.confirm_close_rect();
        let inner = Rect::new(
            popup.x + 1,
            popup.y + 1,
            popup.width.saturating_sub(2),
            popup.height.saturating_sub(2),
        );
        let (confirm, _) = crate::ui::confirm_close_button_rects(inner);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            confirm.x + 1,
            confirm.y,
        ));

        assert_eq!(app.state.workspaces.len(), 1);
        assert_eq!(app.state.workspaces[0].display_name(), "a");
    }

    #[tokio::test]
    async fn keyboard_context_menu_split_keeps_new_runtime() {
        let mut app = app_for_mouse_test();
        app.state.default_shell = "/usr/bin/true".into();
        let (workspace, terminal, runtime) = Workspace::new(
            std::env::current_dir().unwrap_or_else(|_| "/".into()),
            24,
            80,
            app.state.pane_scrollback_limit_bytes,
            app.state.host_terminal_theme,
            crate::pane::PaneShellConfig::new(&app.state.default_shell, app.state.shell_mode),
            app.event_tx.clone(),
            app.render_notify.clone(),
            app.render_dirty.clone(),
        )
        .expect("workspace should spawn");
        app.state.workspaces = vec![workspace];
        app.terminal_runtimes.insert(terminal.id.clone(), runtime);
        app.state.terminals.insert(terminal.id.clone(), terminal);
        app.state.active = Some(0);
        app.state.selected = 0;
        let pane_id = app.state.workspaces[0].tabs[0].root_pane;
        let runtime_count = app.terminal_runtimes.len();
        app.state.context_menu = Some(ContextMenuState {
            kind: ContextMenuKind::Pane {
                pane_id,
                has_manual_label: false,
            },
            x: 2,
            y: 2,
            list: MenuListState::new(1),
        });
        app.state.mode = Mode::ContextMenu;

        handle_context_menu_key(
            &mut app.state,
            &mut app.terminal_runtimes,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );

        assert_eq!(app.state.mode, Mode::Terminal);
        assert_eq!(app.state.workspaces[0].tabs[0].layout.pane_count(), 2);
        assert_eq!(app.terminal_runtimes.len(), runtime_count + 1);

        let runtimes: Vec<_> = app.terminal_runtimes.drain().collect();
        for (_terminal_id, runtime) in runtimes {
            runtime.shutdown();
        }
    }

    #[test]
    fn dragging_pane_split_updates_captured_layout_ratio() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("test")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.workspaces[0].test_split(Direction::Horizontal);
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let border = app.state.view.split_borders[0].clone();
        let before = capture_snapshot(&app.state);
        let drag_row = border.area.y.saturating_add(1);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            border.pos,
            drag_row,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            border.pos.saturating_add(6),
            drag_row,
        ));

        let after = capture_snapshot(&app.state);
        assert_ne!(root_layout_ratio(&before), root_layout_ratio(&after));
    }

    #[test]
    fn pane_split_hitbox_does_not_overlap_right_pane_content() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("test")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.workspaces[0].test_split(Direction::Horizontal);
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let border = app.state.view.split_borders[0].clone();
        let row = border.area.y.saturating_add(1);

        assert!(app
            .state
            .find_border_at(border.pos.saturating_sub(1), row)
            .is_some());
        assert!(app.state.find_border_at(border.pos, row).is_some());
        assert!(app
            .state
            .find_border_at(border.pos.saturating_add(1), row)
            .is_none());
    }

    #[test]
    fn pane_split_hitbox_does_not_overlap_bottom_pane_content() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("test")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.workspaces[0].test_split(Direction::Vertical);
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let border = app.state.view.split_borders[0].clone();
        let col = border.area.x.saturating_add(1);

        assert!(app
            .state
            .find_border_at(col, border.pos.saturating_sub(1))
            .is_some());
        assert!(app.state.find_border_at(col, border.pos).is_some());
        assert!(app
            .state
            .find_border_at(col, border.pos.saturating_add(1))
            .is_none());
    }

    #[test]
    fn selecting_from_right_pane_first_content_column_starts_selection() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let second_pane = ws.test_split(Direction::Horizontal);
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let second_info = app
            .state
            .view
            .pane_infos
            .iter()
            .find(|info| info.id == second_pane)
            .expect("second pane info")
            .clone();
        let col = second_info.inner_rect.x;
        let row = second_info.inner_rect.y;

        assert!(app.state.find_border_at(col, row).is_none());
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), col, row));

        assert!(app.state.drag.is_none());
        assert_eq!(
            app.state
                .selection
                .as_ref()
                .map(|selection| selection.pane_id),
            Some(second_pane)
        );
    }

    #[test]
    fn selecting_from_bottom_pane_first_content_row_starts_selection() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let second_pane = ws.test_split(Direction::Vertical);
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let second_info = app
            .state
            .view
            .pane_infos
            .iter()
            .find(|info| info.id == second_pane)
            .expect("second pane info")
            .clone();
        let col = second_info.inner_rect.x;
        let row = second_info.inner_rect.y;

        assert!(app.state.find_border_at(col, row).is_none());
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), col, row));

        assert!(app.state.drag.is_none());
        assert_eq!(
            app.state
                .selection
                .as_ref()
                .map(|selection| selection.pane_id),
            Some(second_pane)
        );
    }

    #[tokio::test]
    async fn dragging_vertical_pane_split_still_resizes_when_pane_mouse_reporting_is_enabled() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let first_pane = ws.tabs[0].root_pane;
        let second_pane = ws.test_split(Direction::Vertical);

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let pane_infos = app.state.view.pane_infos.clone();
        let first_info = pane_infos
            .iter()
            .find(|info| info.id == first_pane)
            .expect("first pane info")
            .clone();
        let second_info = pane_infos
            .iter()
            .find(|info| info.id == second_pane)
            .expect("second pane info")
            .clone();

        app.state.insert_test_runtime(
            first_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(
                first_info.inner_rect.width.max(1),
                first_info.inner_rect.height.max(1),
                b"\x1b[?1002h",
            ),
        );
        app.state.insert_test_runtime(
            second_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(
                second_info.inner_rect.width.max(1),
                second_info.inner_rect.height.max(1),
                b"\x1b[?1002h",
            ),
        );

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let border = app
            .state
            .view
            .split_borders
            .iter()
            .find(|border| border.direction == Direction::Vertical)
            .expect("vertical split border")
            .clone();
        let before = capture_snapshot(&app.state);
        let drag_col = border.area.x.saturating_add(1);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            drag_col,
            border.pos,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            drag_col,
            border.pos.saturating_add(4),
        ));

        let after = capture_snapshot(&app.state);
        assert_ne!(root_layout_ratio(&before), root_layout_ratio(&after));
    }

    #[tokio::test]
    async fn dragging_horizontal_pane_split_still_resizes_when_pane_mouse_reporting_is_enabled() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let first_pane = ws.tabs[0].root_pane;
        let second_pane = ws.test_split(Direction::Horizontal);

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let pane_infos = app.state.view.pane_infos.clone();
        let first_info = pane_infos
            .iter()
            .find(|info| info.id == first_pane)
            .expect("first pane info")
            .clone();
        let second_info = pane_infos
            .iter()
            .find(|info| info.id == second_pane)
            .expect("second pane info")
            .clone();

        app.state.insert_test_runtime(
            first_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(
                first_info.inner_rect.width.max(1),
                first_info.inner_rect.height.max(1),
                b"\x1b[?1002h",
            ),
        );
        app.state.insert_test_runtime(
            second_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(
                second_info.inner_rect.width.max(1),
                second_info.inner_rect.height.max(1),
                b"\x1b[?1002h",
            ),
        );

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let border = app
            .state
            .view
            .split_borders
            .iter()
            .find(|border| border.direction == Direction::Horizontal)
            .expect("horizontal split border")
            .clone();
        let before = capture_snapshot(&app.state);
        let drag_row = border.area.y.saturating_add(1);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            border.pos,
            drag_row,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            border.pos.saturating_add(6),
            drag_row,
        ));

        let after = capture_snapshot(&app.state);
        assert_ne!(root_layout_ratio(&before), root_layout_ratio(&after));
    }

    #[test]
    fn wheel_routing_prefers_mouse_reporting() {
        let input_state = crate::pane::InputState {
            alternate_screen: true,
            application_cursor: false,
            bracketed_paste: false,
            focus_reporting: false,
            mouse_protocol_mode: crate::input::MouseProtocolMode::ButtonMotion,
            mouse_protocol_encoding: crate::input::MouseProtocolEncoding::Sgr,
            mouse_alternate_scroll: true,
            modify_other_keys: false,
        };

        assert_eq!(wheel_routing(input_state), WheelRouting::MouseReport);
    }

    #[test]
    fn wheel_over_tab_bar_switches_tabs() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("one");
        ws.test_add_tab(Some("two"));
        ws.test_add_tab(Some("three"));
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let tab_bar = app.state.view.tab_bar_rect;

        app.handle_mouse(mouse(MouseEventKind::ScrollDown, tab_bar.x + 1, tab_bar.y));
        assert_eq!(app.state.workspaces[0].active_tab, 1);

        app.handle_mouse(mouse(MouseEventKind::ScrollUp, tab_bar.x + 1, tab_bar.y));
        assert_eq!(app.state.workspaces[0].active_tab, 0);

        app.handle_mouse(mouse(MouseEventKind::ScrollUp, tab_bar.x + 1, tab_bar.y));
        assert_eq!(app.state.workspaces[0].active_tab, 2);

        app.handle_mouse(mouse(
            MouseEventKind::ScrollDown,
            tab_bar.x + tab_bar.width.saturating_sub(1),
            tab_bar.y,
        ));
        assert_eq!(app.state.workspaces[0].active_tab, 0);
    }

    #[test]
    fn wheel_over_overflowing_tab_bar_switches_tabs() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("one");
        ws.tabs[0].set_custom_name("very-long-one".into());
        ws.test_add_tab(Some("very-long-two"));
        ws.test_add_tab(Some("very-long-three"));
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 65, 20));
        assert!(app.state.view.tab_scroll_right_hit_area.width > 0);
        let tab_bar = app.state.view.tab_bar_rect;

        app.handle_mouse(mouse(
            MouseEventKind::ScrollDown,
            tab_bar.x + tab_bar.width.saturating_sub(2),
            tab_bar.y,
        ));
        assert_eq!(app.state.workspaces[0].active_tab, 1);

        app.handle_mouse(mouse(
            MouseEventKind::ScrollDown,
            tab_bar.x + tab_bar.width.saturating_sub(2),
            tab_bar.y,
        ));
        assert_eq!(app.state.workspaces[0].active_tab, 2);
    }

    #[test]
    fn wheel_outside_tab_bar_does_not_switch_tabs() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("one");
        ws.test_add_tab(Some("two"));
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let terminal = app.state.view.terminal_area;

        app.handle_mouse(mouse(
            MouseEventKind::ScrollDown,
            terminal.x + 1,
            terminal.y + 1,
        ));

        assert_eq!(app.state.workspaces[0].active_tab, 0);
    }

    #[test]
    fn mobile_switch_button_opens_switcher_and_workspace_row_switches_workspace() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("one"), Workspace::test_new("two")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 44, 20));
        assert_eq!(app.state.view.layout, ViewLayout::Mobile);

        let switch = app.state.view.mobile_menu_hit_area;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            switch.x + 1,
            switch.y + 1,
        ));

        assert_eq!(app.state.mode, Mode::Navigate);

        let viewport = crate::ui::mobile_switcher_areas(&app.state).viewport;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            viewport.x + 2,
            viewport.y + 6,
        ));

        assert_eq!(app.state.active, Some(1));
        assert_eq!(app.state.mode, Mode::Terminal);
    }

    #[test]
    fn mobile_workspace_panel_scroll_reaches_extra_workspaces() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = (0..12)
            .map(|idx| Workspace::test_new(&format!("ws-{idx}")))
            .collect();
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 44, 20));
        let switch = app.state.view.mobile_menu_hit_area;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            switch.x + 1,
            switch.y + 1,
        ));
        assert_eq!(app.state.mode, Mode::Navigate);

        let viewport = crate::ui::mobile_switcher_areas(&app.state).viewport;
        app.handle_mouse(mouse(
            MouseEventKind::ScrollDown,
            viewport.x + 2,
            viewport.y,
        ));
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 44, 20));
        assert_eq!(app.state.mobile_switcher_scroll, 2);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            viewport.x + 2,
            viewport.y + 4,
        ));

        assert_eq!(app.state.active, Some(1));
        assert_eq!(app.state.mode, Mode::Terminal);
    }

    #[test]
    fn mobile_global_scroll_reaches_tabs_and_switches_tab() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("one");
        ws.test_add_tab(Some("two"));
        ws.test_add_tab(Some("three"));
        ws.test_add_tab(Some("four"));
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 44, 12));
        let switch = app.state.view.mobile_menu_hit_area;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            switch.x + 1,
            switch.y + 1,
        ));

        let viewport = crate::ui::mobile_switcher_areas(&app.state).viewport;

        app.handle_mouse(mouse(
            MouseEventKind::ScrollDown,
            viewport.x + 2,
            viewport.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::ScrollDown,
            viewport.x + 2,
            viewport.y,
        ));
        assert_eq!(app.state.mobile_switcher_scroll, 4);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            viewport.x + 2,
            viewport.y + 6,
        ));
        assert_eq!(app.state.workspaces[0].active_tab, 2);
    }

    #[test]
    fn mobile_switcher_action_rows_create_workspace_and_open_tab_dialog() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("one");
        ws.test_add_tab(Some("logs"));
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 44, 20));
        let switch = app.state.view.mobile_menu_hit_area;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            switch.x + 1,
            switch.y + 1,
        ));
        let viewport = crate::ui::mobile_switcher_areas(&app.state).viewport;

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            viewport.x + 2,
            viewport.y + 3,
        ));
        assert!(app.state.request_new_workspace);

        app.state.request_new_workspace = false;
        app.state.mode = Mode::Navigate;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            viewport.x + 2,
            viewport.y + 7,
        ));
        assert_eq!(app.state.mode, Mode::RenameTab);
        assert!(app.state.creating_new_tab);
    }

    #[test]
    fn mobile_switcher_swallows_non_left_mouse_events() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("one")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 44, 20));
        let switch = app.state.view.mobile_menu_hit_area;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            switch.x + 1,
            switch.y + 1,
        ));
        assert_eq!(app.state.mode, Mode::Navigate);

        let viewport = crate::ui::mobile_switcher_areas(&app.state).viewport;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            viewport.x + 2,
            viewport.y + 2,
        ));

        assert_eq!(app.state.mode, Mode::Navigate);
        assert!(app.state.context_menu.is_none());
    }

    #[test]
    fn mobile_switch_button_does_not_bypass_rename_modal() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("one")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::RenameTab;
        app.state.creating_new_tab = true;
        app.state.name_input = "new tab".into();

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 44, 20));
        let switch = app.state.view.mobile_menu_hit_area;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            switch.x + 1,
            switch.y + 1,
        ));

        assert_eq!(app.state.mode, Mode::Terminal);
        assert!(!app.state.creating_new_tab);
        assert!(!app.state.request_new_tab);
    }

    #[test]
    fn mobile_switcher_close_returns_to_terminal() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("one")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 44, 20));
        let switch = app.state.view.mobile_menu_hit_area;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            switch.x + 1,
            switch.y + 1,
        ));
        assert_eq!(app.state.mode, Mode::Navigate);

        let close = crate::ui::mobile_switcher_areas(&app.state).close;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            close.x + 1,
            close.y,
        ));

        assert_eq!(app.state.mode, Mode::Terminal);
    }

    #[test]
    fn wheel_routing_uses_alternate_scroll_in_fullscreen_without_mouse_reporting() {
        let input_state = crate::pane::InputState {
            alternate_screen: true,
            application_cursor: false,
            bracketed_paste: false,
            focus_reporting: false,
            mouse_protocol_mode: crate::input::MouseProtocolMode::None,
            mouse_protocol_encoding: crate::input::MouseProtocolEncoding::Default,
            mouse_alternate_scroll: true,
            modify_other_keys: false,
        };

        assert_eq!(wheel_routing(input_state), WheelRouting::AlternateScroll);
    }

    #[test]
    fn wheel_routing_falls_back_to_host_scrollback() {
        let input_state = crate::pane::InputState {
            alternate_screen: false,
            application_cursor: false,
            bracketed_paste: false,
            focus_reporting: false,
            mouse_protocol_mode: crate::input::MouseProtocolMode::None,
            mouse_protocol_encoding: crate::input::MouseProtocolEncoding::Default,
            mouse_alternate_scroll: true,
            modify_other_keys: false,
        };

        assert_eq!(wheel_routing(input_state), WheelRouting::HostScroll);
    }

    #[test]
    fn kanban_global_menu_and_settings_mouse() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("one")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Kanban;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 80, 20));
        let launcher = app.state.global_launcher_rect();

        // 1. Click launcher to open global launcher menu
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            launcher.x + 1,
            launcher.y,
        ));
        assert_eq!(app.state.mode, Mode::GlobalMenu);
        assert_eq!(app.state.prefix_previous_mode, Some(Mode::Kanban));

        // 2. Select Settings from launcher menu
        let labels = app.state.global_menu_labels();
        let settings_idx = labels.iter().position(|l| *l == "settings").unwrap();
        app.state.global_menu.highlighted = settings_idx;
        let menu_rect = app.state.global_menu_rect();
        // Click on settings menu item
        let click_y = menu_rect.y + 1 + settings_idx as u16;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            menu_rect.x + 2,
            click_y,
        ));

        assert_eq!(app.state.mode, Mode::Settings);
        assert_eq!(app.state.prefix_previous_mode, Some(Mode::Kanban));

        // 3. Click Close/Cancel on Settings UI
        let inner = app.state.settings_inner_rect();
        let show_primary = crate::ui::settings_show_primary_action(&app.state);
        let (_, close_rect) =
            crate::ui::settings_button_rects(inner, app.state.settings.section, show_primary);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            close_rect.x + 1,
            close_rect.y,
        ));

        // Exiting settings should return us back to Kanban mode
        assert_eq!(app.state.mode, Mode::Kanban);
        assert_eq!(app.state.prefix_previous_mode, None);
    }

    #[test]
    fn kanban_card_click_and_scroll() {
        let mut app = app_for_mouse_test();
        let ws = Workspace::test_new("one");
        let pane_id = ws.tabs[0].root_pane;
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Kanban;

        // Add an item with terminal_id
        let term_id_str = app.state.workspaces[0].tabs[0].panes[&pane_id]
            .attached_terminal_id
            .to_string();
        app.state.kanban_items.clear();
        app.state.add_kanban_item(
            "Tracked Task".to_string(),
            None,
            Some(crate::api::schema::KanbanStatus::Todo),
            Some(term_id_str),
        );

        app.state.view.terminal_area = Rect::new(26, 0, 84, 20);

        // 1. Click on the first card
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 30, 3));

        // Click should switch mode to Terminal and focus the pane
        assert_eq!(app.state.mode, Mode::Terminal);

        // Reset to Kanban mode and right click the card
        app.state.mode = Mode::Kanban;
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 30, 3));

        // Right click should stay in Kanban mode and open the detailed modal
        assert_eq!(app.state.mode, Mode::Kanban);
        let tracked_item = app.state.kanban_items.first().unwrap().clone();
        assert_eq!(app.state.kanban_detail_uuid, Some(tracked_item.uuid));
        // Insert active terminal state to make the terminal clickable/jumpable
        let terminal_state = crate::terminal::TerminalState::new(
            app.state.workspaces[0].tabs[0].panes[&pane_id]
                .attached_terminal_id
                .clone(),
            std::path::PathBuf::from("/tmp"),
        );
        app.state.terminals.insert(
            app.state.workspaces[0].tabs[0].panes[&pane_id]
                .attached_terminal_id
                .clone(),
            terminal_state,
        );

        // Adjust terminal area size to 100x30 to make the centered popup calculations predictable
        app.state.view.terminal_area = Rect::new(0, 0, 100, 30);

        // Click on the terminal row (y = 9, x = 20) inside the popup
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 20, 9));

        // Click should jump to Terminal mode, focus the pane, and close detail modal
        assert_eq!(app.state.mode, Mode::Terminal);
        assert_eq!(app.state.kanban_detail_uuid, None);

        // Restore terminal area for the rest of the test
        app.state.view.terminal_area = Rect::new(26, 0, 84, 20);

        // Reset to Kanban mode and clear pane_id to test fallback to detail modal
        app.state.mode = Mode::Kanban;
        app.state.kanban_items.clear();
        let item = app.state.add_kanban_item(
            "Fallback Task".to_string(),
            None,
            Some(crate::api::schema::KanbanStatus::Todo),
            None,
        );

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 30, 3));
        assert_eq!(app.state.mode, Mode::Kanban);
        assert_eq!(app.state.kanban_detail_uuid, Some(item.uuid.clone()));

        // Click outside the modal to close it
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            1, // in sidebar area, which is outside detailed modal
            1,
        ));
        assert_eq!(app.state.kanban_detail_uuid, None);

        // Test scrolling over Kanban columns
        app.state.kanban_items.clear();
        for i in 0..5 {
            app.state.add_kanban_item(
                format!("Task {i}"),
                None,
                Some(crate::api::schema::KanbanStatus::Todo),
                None,
            );
        }
        app.state.kanban_selected_col = 0;
        app.state.kanban_selected_row = 0;

        // Scroll down
        app.handle_mouse(mouse(MouseEventKind::ScrollDown, 30, 3));
        assert_eq!(app.state.kanban_selected_col, 0);
        assert_eq!(app.state.kanban_selected_row, 1);

        // Scroll up
        app.handle_mouse(mouse(MouseEventKind::ScrollUp, 30, 3));
        assert_eq!(app.state.kanban_selected_col, 0);
        assert_eq!(app.state.kanban_selected_row, 0);

        // Test scrolling over Kanban columns in Mobile layout (vertical splits/rows)
        app.state.view.layout = ViewLayout::Mobile;
        let old_sidebar = app.state.view.sidebar_rect;
        app.state.view.sidebar_rect = Rect::default();
        app.state.view.terminal_area = Rect::new(0, 0, 80, 20); // y from 0 to 20
                                                                // Four rows/vertical columns, each gets 5 rows:
                                                                // Todo: y=0..5, InProgress: y=5..10, NeedReview: y=10..15, Done: y=15..20

        app.state.kanban_items.clear();
        for i in 0..5 {
            app.state.add_kanban_item(
                format!("Task {i}"),
                None,
                Some(crate::api::schema::KanbanStatus::InProgress), // Column 1
                None,
            );
        }
        app.state.kanban_selected_col = 0;
        app.state.kanban_selected_row = 0;

        // Scroll down on In Progress (hover at y = 7, inside y = 5..10)
        app.handle_mouse(mouse(MouseEventKind::ScrollDown, 10, 7));
        assert_eq!(app.state.kanban_selected_col, 1);
        assert_eq!(app.state.kanban_selected_row, 1);

        // Scroll up on In Progress (hover at y = 7)
        app.handle_mouse(mouse(MouseEventKind::ScrollUp, 10, 7));
        assert_eq!(app.state.kanban_selected_col, 1);
        assert_eq!(app.state.kanban_selected_row, 0);

        // Reset layout to Desktop
        app.state.view.layout = ViewLayout::Desktop;
        app.state.view.sidebar_rect = old_sidebar;

        // Test scrolling when detailed modal is open
        let temp_dir = std::env::temp_dir();
        let plan_file = temp_dir.join(format!("herdr-test-mouse-{}.md", uuid::Uuid::new_v4()));
        let desc_text = "A very long description. ".repeat(50);
        std::fs::write(&plan_file, &desc_text).unwrap();
        let plan_path = plan_file.to_string_lossy().to_string();

        let item = app.state.add_kanban_item(
            "Scroll Task".to_string(),
            Some(plan_path.clone()),
            Some(crate::api::schema::KanbanStatus::Todo),
            None,
        );
        app.state.set_kanban_detail_uuid(Some(item.uuid.clone()));
        app.state.view.terminal_area = Rect::new(0, 0, 80, 24);

        assert_eq!(app.state.kanban_detail_scroll, 0);
        let max_scroll = app.state.kanban_detail_max_scroll();
        assert!(max_scroll > 0);

        // Scroll down in modal using mouse wheel
        app.handle_mouse(mouse(MouseEventKind::ScrollDown, 40, 10));
        assert_eq!(app.state.kanban_detail_scroll, 3);

        // Scroll up in modal using mouse wheel
        app.handle_mouse(mouse(MouseEventKind::ScrollUp, 40, 10));
        assert_eq!(app.state.kanban_detail_scroll, 0);

        // Click outside (e.g. sidebar col 1) should close detailed view
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 1, 1));
        assert_eq!(app.state.kanban_detail_uuid, None);

        // Test clicking footer buttons inside detailed modal
        app.state.kanban_items.clear();
        app.state.kanban_selected_col = 0;
        app.state.kanban_selected_row = 0;
        let item = app.state.add_kanban_item(
            "Button Click Task".to_string(),
            None,
            Some(crate::api::schema::KanbanStatus::Todo),
            None,
        );
        app.state.set_kanban_detail_uuid(Some(item.uuid.clone()));
        app.state.view.terminal_area = Rect::new(0, 0, 80, 24);

        // Click Copy UUID button (x = 40, y = 20)
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 40, 20));
        assert_eq!(app.state.kanban_detail_uuid, Some(item.uuid.clone()));
        let event = app.event_rx.try_recv().unwrap();
        assert!(matches!(
            event,
            crate::events::AppEvent::ClipboardWrite { content } if content == item.uuid.clone().into_bytes()
        ));

        // Click Close Details button (x = 15, y = 20)
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 15, 20));
        assert_eq!(app.state.kanban_detail_uuid, None);

        // Re-open and Click Delete Card button (x = 60, y = 20)
        app.state.set_kanban_detail_uuid(Some(item.uuid.clone()));
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 60, 20));
        assert_eq!(app.state.kanban_detail_uuid, None);
        assert!(app.state.kanban_items.is_empty());

        // Test clicking hyperlinks inside detailed modal
        let link_file = temp_dir.join(format!("herdr-test-link-{}.md", uuid::Uuid::new_v4()));
        let link_desc =
            "Check [my file](file:///path/to/my/file.txt) and [my website](https://example.com).";
        std::fs::write(&link_file, &link_desc).unwrap();
        let link_path = link_file.to_string_lossy().to_string();

        let item = app.state.add_kanban_item(
            "Link Test Task".to_string(),
            Some(link_path.clone()),
            Some(crate::api::schema::KanbanStatus::Todo),
            None,
        );
        app.state.set_kanban_detail_uuid(Some(item.uuid.clone()));
        app.state.view.terminal_area = Rect::new(0, 0, 80, 24);

        let links = app.state.active_kanban_detail_hyperlinks();
        let file_link_coord = links
            .iter()
            .find(|(_, _, url)| url == "file:///path/to/my/file.txt")
            .map(|(coord, _, _)| *coord);
        let web_link_coord = links
            .iter()
            .find(|(_, _, url)| url == "https://example.com")
            .map(|(coord, _, _)| *coord);

        assert!(file_link_coord.is_some());
        assert!(web_link_coord.is_some());

        let (fx, fy) = file_link_coord.unwrap();
        let (wx, wy) = web_link_coord.unwrap();

        // Clicking the file link should trigger a clipboard write event
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), fx, fy));
        let event = app.event_rx.try_recv().unwrap();
        assert!(matches!(
            event,
            crate::events::AppEvent::ClipboardWrite { content } if content == b"file:///path/to/my/file.txt".to_vec()
        ));

        // Clicking the web link should NOT trigger a clipboard write event
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), wx, wy));
        assert!(app.event_rx.try_recv().is_err());

        // Cleanup
        let _ = std::fs::remove_file(link_file);
        let _ = std::fs::remove_file(plan_file);
    }
}
