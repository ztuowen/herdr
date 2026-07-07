use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::{AppState, Mode};
use crate::input::TerminalKey;

#[allow(clippy::collapsible_match)] // Allow nested if in match arm to avoid side-effects in match guards
pub(crate) fn handle_kanban_key(state: &mut AppState, key: TerminalKey) {
    if state.is_prefix_key(key) {
        state.prefix_previous_mode = Some(Mode::Kanban);
        state.mode = Mode::Prefix;
        return;
    }

    if state.keybinds.toggle_kanban.matches_direct_key(key) {
        if state.mode == Mode::Kanban {
            if state.active.is_some() {
                state.mode = Mode::Terminal;
            } else {
                state.mode = Mode::Navigate;
            }
        } else {
            state.mode = Mode::Kanban;
        }
        return;
    }

    let key_event = key.as_key_event();
    let board_layout = crate::extensions::kanban::ui::kanban_board_layout(state);
    if state.extensions.kanban.detail_uuid.is_some() {
        match key_event.code {
            KeyCode::Esc => {
                state.extensions.kanban.set_detail_uuid(None);
            }
            KeyCode::Char('c') | KeyCode::Char('y') => {
                if let Some(ref uuid) = state.extensions.kanban.detail_uuid {
                    state.request_clipboard_write = Some(uuid.clone().into_bytes());
                }
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                state.extensions.kanban.delete_selected();
                state.mark_session_dirty();
                state.extensions.kanban.set_detail_uuid(None);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let max_scroll = crate::extensions::kanban::ui::kanban_detail_max_scroll(state);
                state.extensions.kanban.scroll_detail(-1, max_scroll);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max_scroll = crate::extensions::kanban::ui::kanban_detail_max_scroll(state);
                state.extensions.kanban.scroll_detail(1, max_scroll);
            }
            KeyCode::Left | KeyCode::Char('h') => {
                let max_scroll =
                    crate::extensions::kanban::ui::kanban_detail_max_horizontal_scroll(state);
                state
                    .extensions
                    .kanban
                    .scroll_horizontal_detail(-2, max_scroll);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let max_scroll =
                    crate::extensions::kanban::ui::kanban_detail_max_horizontal_scroll(state);
                state
                    .extensions
                    .kanban
                    .scroll_horizontal_detail(2, max_scroll);
            }
            KeyCode::PageUp => {
                let max_scroll = crate::extensions::kanban::ui::kanban_detail_max_scroll(state);
                state.extensions.kanban.scroll_detail(-10, max_scroll);
            }
            KeyCode::PageDown => {
                let max_scroll = crate::extensions::kanban::ui::kanban_detail_max_scroll(state);
                state.extensions.kanban.scroll_detail(10, max_scroll);
            }
            _ => {}
        }
        return;
    }

    match key_event.code {
        KeyCode::Esc => {
            leave_kanban(state);
        }
        KeyCode::Char('c') | KeyCode::Char('y') => {
            if let crate::extensions::kanban::KanbanBoardAction::CopyUuid { uuid } =
                state.extensions.kanban.copy_selected_uuid()
            {
                state.request_clipboard_write = Some(uuid.into_bytes());
            }
        }
        KeyCode::Left
            if key_event.modifiers == KeyModifiers::SHIFT
                && board_layout == crate::extensions::kanban::KanbanBoardLayout::Desktop
                && state.extensions.kanban.shift_selected_item_for_layout(
                    board_layout,
                    crate::extensions::kanban::KanbanBoardDirection::Left,
                ) =>
        {
            state.mark_session_dirty();
        }
        KeyCode::Char('H')
            if state.extensions.kanban.shift_selected_item_for_layout(
                board_layout,
                crate::extensions::kanban::KanbanBoardDirection::Left,
            ) =>
        {
            state.mark_session_dirty();
        }
        KeyCode::Right
            if key_event.modifiers == KeyModifiers::SHIFT
                && board_layout == crate::extensions::kanban::KanbanBoardLayout::Desktop
                && state.extensions.kanban.shift_selected_item_for_layout(
                    board_layout,
                    crate::extensions::kanban::KanbanBoardDirection::Right,
                ) =>
        {
            state.mark_session_dirty();
        }
        KeyCode::Char('L')
            if state.extensions.kanban.shift_selected_item_for_layout(
                board_layout,
                crate::extensions::kanban::KanbanBoardDirection::Right,
            ) =>
        {
            state.mark_session_dirty();
        }
        KeyCode::Up
            if key_event.modifiers == KeyModifiers::SHIFT
                && board_layout == crate::extensions::kanban::KanbanBoardLayout::Mobile
                && state.extensions.kanban.shift_selected_item_for_layout(
                    board_layout,
                    crate::extensions::kanban::KanbanBoardDirection::Up,
                ) =>
        {
            state.mark_session_dirty();
        }
        KeyCode::Char('K')
            if state.extensions.kanban.shift_selected_item_for_layout(
                board_layout,
                crate::extensions::kanban::KanbanBoardDirection::Up,
            ) =>
        {
            state.mark_session_dirty();
        }
        KeyCode::Down
            if key_event.modifiers == KeyModifiers::SHIFT
                && board_layout == crate::extensions::kanban::KanbanBoardLayout::Mobile
                && state.extensions.kanban.shift_selected_item_for_layout(
                    board_layout,
                    crate::extensions::kanban::KanbanBoardDirection::Down,
                ) =>
        {
            state.mark_session_dirty();
        }
        KeyCode::Char('J')
            if state.extensions.kanban.shift_selected_item_for_layout(
                board_layout,
                crate::extensions::kanban::KanbanBoardDirection::Down,
            ) =>
        {
            state.mark_session_dirty();
        }
        KeyCode::Left | KeyCode::Char('h') => {
            state.extensions.kanban.move_board_selection(
                board_layout,
                crate::extensions::kanban::KanbanBoardDirection::Left,
            );
        }
        KeyCode::Right | KeyCode::Char('l') => {
            state.extensions.kanban.move_board_selection(
                board_layout,
                crate::extensions::kanban::KanbanBoardDirection::Right,
            );
        }
        KeyCode::Up | KeyCode::Char('k') => {
            state.extensions.kanban.move_board_selection(
                board_layout,
                crate::extensions::kanban::KanbanBoardDirection::Up,
            );
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.extensions.kanban.move_board_selection(
                board_layout,
                crate::extensions::kanban::KanbanBoardDirection::Down,
            );
        }
        KeyCode::Char(' ') | KeyCode::Enter => {
            state.extensions.kanban.open_selected_detail();
        }
        KeyCode::Char('d') | KeyCode::Delete => {
            state.extensions.kanban.delete_selected();
            state.mark_session_dirty();
        }
        _ => {}
    }
}

fn leave_kanban(state: &mut AppState) {
    if let Some(prev) = state.prefix_previous_mode.take() {
        state.mode = prev;
    } else if state.active.is_some() {
        state.mode = Mode::Terminal;
    } else {
        state.mode = Mode::Navigate;
    }
}
