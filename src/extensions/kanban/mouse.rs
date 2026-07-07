use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::app::state::{AppState, Mode, NavigatorTarget};

pub(crate) fn handle_kanban_mouse(state: &mut AppState, mouse: MouseEvent) -> bool {
    if state.mode != Mode::Kanban {
        return false;
    }

    if state.extensions.kanban.detail_uuid.is_some() {
        handle_detail_mouse(state, mouse);
        return true;
    }

    let sidebar = state.view.sidebar_rect;
    let in_sidebar = sidebar.width > 0
        && mouse.column >= sidebar.x
        && mouse.column < sidebar.x + sidebar.width
        && mouse.row >= sidebar.y
        && mouse.row < sidebar.y + sidebar.height;

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Down(MouseButton::Right)
            if !in_sidebar =>
        {
            let board_layout = crate::extensions::kanban::ui::kanban_board_layout(state);
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Right)) {
                state.extensions.kanban.open_card_at(
                    state.view.terminal_area,
                    board_layout,
                    mouse.column,
                    mouse.row,
                );
            } else if let crate::extensions::kanban::KanbanBoardAction::ActivateCard {
                uuid,
                terminal_id,
            } = state.extensions.kanban.activate_card_at(
                state.view.terminal_area,
                board_layout,
                mouse.column,
                mouse.row,
            ) {
                let mut navigated = false;
                if let Some(ref term_id_str) = terminal_id {
                    if let Some((ws_idx, tab_idx, pane_id)) =
                        state.find_pane_by_terminal_id_str(term_id_str)
                    {
                        state.focus_navigator_target(NavigatorTarget::Pane {
                            ws_idx,
                            tab_idx,
                            pane_id,
                        });
                        navigated = true;
                    }
                }
                if !navigated {
                    state.extensions.kanban.set_detail_uuid(Some(uuid));
                }
            }
            return true;
        }
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown if !in_sidebar => {
            let delta = if matches!(mouse.kind, MouseEventKind::ScrollUp) {
                -1
            } else {
                1
            };
            state.extensions.kanban.scroll_board_at(
                state.view.terminal_area,
                crate::extensions::kanban::ui::kanban_board_layout(state),
                mouse.column,
                mouse.row,
                delta,
            );
            return true;
        }
        _ => {}
    }

    !in_sidebar
}

fn handle_detail_mouse(state: &mut AppState, mouse: MouseEvent) {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => handle_detail_click(state, mouse),
        MouseEventKind::ScrollUp => {
            let max_scroll = crate::extensions::kanban::ui::kanban_detail_max_scroll(state);
            state.extensions.kanban.scroll_detail(-3, max_scroll);
        }
        MouseEventKind::ScrollDown => {
            let max_scroll = crate::extensions::kanban::ui::kanban_detail_max_scroll(state);
            state.extensions.kanban.scroll_detail(3, max_scroll);
        }
        MouseEventKind::ScrollLeft => {
            let max_scroll =
                crate::extensions::kanban::ui::kanban_detail_max_horizontal_scroll(state);
            state
                .extensions
                .kanban
                .scroll_horizontal_detail(-2, max_scroll);
        }
        MouseEventKind::ScrollRight => {
            let max_scroll =
                crate::extensions::kanban::ui::kanban_detail_max_horizontal_scroll(state);
            state
                .extensions
                .kanban
                .scroll_horizontal_detail(2, max_scroll);
        }
        _ => {}
    }
}

fn handle_detail_click(state: &mut AppState, mouse: MouseEvent) {
    let (width, height) =
        crate::extensions::kanban::ui::kanban_detail_modal_size(state.view.terminal_area);
    let Some(rect) = crate::ui::centered_popup_rect(state.view.terminal_area, width, height) else {
        state.extensions.kanban.set_detail_uuid(None);
        return;
    };

    let inside = mouse.column >= rect.x
        && mouse.column < rect.x + rect.width
        && mouse.row >= rect.y
        && mouse.row < rect.y + rect.height;
    if !inside {
        state.extensions.kanban.set_detail_uuid(None);
        return;
    }
    if rect.height < 9 {
        return;
    }

    let inner = Rect::new(
        rect.x + 1,
        rect.y + 1,
        rect.width.saturating_sub(2),
        rect.height.saturating_sub(2),
    );
    let rows = ratatui::layout::Layout::vertical([
        ratatui::layout::Constraint::Length(1),
        ratatui::layout::Constraint::Length(1),
        ratatui::layout::Constraint::Length(1),
        ratatui::layout::Constraint::Length(1),
        ratatui::layout::Constraint::Length(1),
        ratatui::layout::Constraint::Length(1),
        ratatui::layout::Constraint::Min(1),
        ratatui::layout::Constraint::Length(1),
    ])
    .split(inner);

    if handle_detail_hyperlink_click(state, mouse) {
        return;
    }
    handle_detail_terminal_click(state, mouse, rows[5]);
    handle_detail_footer_click(state, mouse, rows[7]);
}

fn handle_detail_hyperlink_click(state: &mut AppState, mouse: MouseEvent) -> bool {
    let Some((_, _, url)) = crate::extensions::kanban::ui::active_kanban_detail_hyperlinks(state)
        .into_iter()
        .find(|((x, y), _, _)| *x == mouse.column && *y == mouse.row)
    else {
        return false;
    };

    if !url.starts_with("http://") && !url.starts_with("https://") {
        state.request_clipboard_write = Some(url.into_bytes());
    } else {
        #[cfg(not(test))]
        if let Err(err) = crate::platform::open_url(&url) {
            tracing::warn!("failed to open markdown link: {err}");
        }
    }
    true
}

fn handle_detail_terminal_click(state: &mut AppState, mouse: MouseEvent, terminal_row: Rect) {
    if mouse.row != terminal_row.y {
        return;
    }
    let Some(uuid) = state.extensions.kanban.detail_uuid.clone() else {
        return;
    };
    let Some(item) = state
        .extensions
        .kanban
        .items
        .iter()
        .find(|it| it.uuid == uuid)
    else {
        return;
    };
    let Some(tid) = item.terminal_id.clone() else {
        return;
    };
    let status = state.kanban_pane_status(&tid);
    if !status.exists {
        return;
    }
    if let Some((ws_idx, tab_idx, pane_id)) = state.find_pane_by_terminal_id_str(&tid) {
        state.focus_navigator_target(NavigatorTarget::Pane {
            ws_idx,
            tab_idx,
            pane_id,
        });
        state.extensions.kanban.set_detail_uuid(None);
    }
}

fn handle_detail_footer_click(state: &mut AppState, mouse: MouseEvent, footer_row: Rect) {
    if mouse.row != footer_row.y {
        return;
    }
    let text_len = 54;
    let x_start = footer_row.x + (footer_row.width.saturating_sub(text_len)) / 2;
    if mouse.column < x_start || mouse.column >= x_start + text_len {
        return;
    }

    let offset = mouse.column - x_start;
    if offset < 22 {
        state.extensions.kanban.set_detail_uuid(None);
    } else if offset < 38 {
        if let Some(ref uuid) = state.extensions.kanban.detail_uuid {
            state.request_clipboard_write = Some(uuid.clone().into_bytes());
        }
    } else {
        state.extensions.kanban.delete_selected();
        state.mark_session_dirty();
        state.extensions.kanban.set_detail_uuid(None);
    }
}
