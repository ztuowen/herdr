// Allow dead code in ui/kanban.rs when the kanban feature is disabled in the build.
#![allow(dead_code)]

use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use super::widgets::{centered_popup_rect, render_panel_shell};
use crate::app::AppState;
use crate::kanban::{KanbanBoardLayout, KanbanBoardProjection};

pub(crate) fn kanban_board_layout(app: &AppState) -> KanbanBoardLayout {
    if app.view.layout == crate::app::state::ViewLayout::Mobile {
        KanbanBoardLayout::Mobile
    } else {
        KanbanBoardLayout::Desktop
    }
}

fn kanban_constraints() -> Vec<Constraint> {
    vec![Constraint::Ratio(1, crate::kanban::column_count() as u32); crate::kanban::column_count()]
}

fn status_label_and_color(
    status: crate::api::schema::KanbanStatus,
    p: &crate::app::state::Palette,
) -> (&'static str, ratatui::style::Color) {
    match status {
        crate::api::schema::KanbanStatus::Todo => ("todo", p.overlay1),
        crate::api::schema::KanbanStatus::Ongoing => ("ongoing", p.yellow),
        crate::api::schema::KanbanStatus::Blocked => ("blocked", p.red),
        crate::api::schema::KanbanStatus::Reviewing => ("reviewing", p.peach),
        crate::api::schema::KanbanStatus::Done => ("done", p.green),
    }
}

pub(crate) fn render_kanban(app: &AppState, frame: &mut Frame, area: Rect) {
    let p = &app.palette;
    let projection = app
        .extensions
        .kanban
        .board_projection(area, kanban_board_layout(app));

    for column in &projection.columns {
        let (name, color) = status_label_and_color(column.status, p);

        // Render column block
        let col_border_color = if column.is_selected {
            p.accent
        } else {
            p.surface0
        };
        let col_title = format!(" {} ({}) ", name.to_uppercase(), column.item_count);
        let col_block = Block::default()
            .borders(Borders::ALL)
            .border_type(if column.is_selected {
                BorderType::Thick
            } else {
                BorderType::Plain
            })
            .border_style(Style::default().fg(col_border_color))
            .title(Span::styled(
                col_title,
                Style::default()
                    .fg(if column.is_selected { p.accent } else { color })
                    .add_modifier(Modifier::BOLD),
            ));

        frame.render_widget(col_block, column.area);

        // Draw cards in this column
        if column.item_count > 0 {
            render_kanban_cards(app, frame, &projection, column.index);
        } else {
            // Draw empty message
            let empty_text = Paragraph::new("No items")
                .style(Style::default().fg(p.overlay0))
                .alignment(Alignment::Center);
            let empty_rect = Rect::new(
                column.inner_area.x,
                column.inner_area.y + column.inner_area.height / 2,
                column.inner_area.width,
                1,
            );
            frame.render_widget(empty_text, empty_rect);
        }
    }

    // Render detailed modal if app.extensions.kanban.detail_uuid is Some
    if let Some(ref uuid) = app.extensions.kanban.detail_uuid {
        if let Some(item) = app
            .extensions
            .kanban
            .items
            .iter()
            .find(|it| it.uuid == *uuid)
        {
            render_kanban_detail_modal(app, frame, area, item);
        }
    }
}

fn render_kanban_cards(
    app: &AppState,
    frame: &mut Frame,
    projection: &KanbanBoardProjection,
    col_idx: usize,
) {
    let Some(column) = projection.columns.get(col_idx) else {
        return;
    };
    let p = &app.palette;
    for card in &column.cards {
        let has_active_terminal = card
            .item
            .terminal_id
            .as_deref()
            .map(|tid| app.kanban_pane_status(tid).exists)
            .unwrap_or(false);

        let card_border_color = if has_active_terminal {
            p.green
        } else {
            p.surface0
        };
        let card_bg = if card.is_selected {
            p.surface0
        } else {
            p.surface_dim
        };

        let border_type = if has_active_terminal {
            BorderType::Thick
        } else {
            BorderType::Plain
        };

        let card_block = Block::default()
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(Style::default().fg(card_border_color))
            .style(Style::default().bg(card_bg));

        let card_inner = card_block.inner(card.area);
        frame.render_widget(card_block, card.area);

        if card_inner.height > 0 {
            let mut card_title_style = Style::default().add_modifier(Modifier::BOLD);
            if has_active_terminal {
                card_title_style = card_title_style.fg(p.green).add_modifier(Modifier::ITALIC);
            } else {
                card_title_style = card_title_style.fg(p.text);
            }

            let formatted_title = format_kanban_title(card.item.title.as_str(), card_inner.width);
            frame.render_widget(
                Paragraph::new(formatted_title).style(card_title_style),
                card_inner,
            );
        }
    }
}

fn render_kanban_detail_modal(
    app: &AppState,
    frame: &mut Frame,
    area: Rect,
    item: &crate::api::schema::KanbanItem,
) {
    let p = &app.palette;
    let (width, height) = kanban_detail_modal_size(area);
    let Some(popup) = centered_popup_rect(area, width, height) else {
        return;
    };

    super::dim_background(frame, area);

    let Some(inner) = render_panel_shell(frame, popup, p.accent, p.panel_bg) else {
        return;
    };

    let rows = Layout::vertical([
        Constraint::Length(1), // Header title
        Constraint::Length(1), // Divider
        Constraint::Length(1), // Card Title
        Constraint::Length(1), // Status Badge
        Constraint::Length(1), // UUID
        Constraint::Length(1), // Associated Terminal
        Constraint::Min(1),    // Description
        Constraint::Length(1), // Footer hint
    ])
    .split(inner);

    // Title
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            " CARD DETAILS",
            Style::default().fg(p.text).add_modifier(Modifier::BOLD),
        )])),
        rows[0],
    );

    // Divider
    let sep = "─".repeat(inner.width as usize);
    frame.render_widget(
        Paragraph::new(Span::styled(&sep, Style::default().fg(p.surface0))),
        rows[1],
    );

    // Title & Status Badge
    let (_, status_color) = status_label_and_color(item.status, p);
    let status_str = item.status.as_str().to_uppercase();

    let title_line = Line::from(vec![Span::styled(
        item.title.as_str(),
        Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
    )]);
    let status_line = Line::from(vec![
        Span::styled("Status: ", Style::default().fg(p.overlay0)),
        Span::styled(
            status_str,
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    frame.render_widget(Paragraph::new(title_line), rows[2]);
    frame.render_widget(Paragraph::new(status_line), rows[3]);

    // UUID
    let uuid_line = Line::from(vec![
        Span::styled("UUID: ", Style::default().fg(p.overlay0)),
        Span::styled(item.uuid.as_str(), Style::default().fg(p.text)),
    ]);
    frame.render_widget(Paragraph::new(uuid_line), rows[4]);

    // Associated Terminal / Pane status
    let terminal_line = if let Some(ref tid) = item.terminal_id {
        let status = app.kanban_pane_status(tid);
        if status.exists {
            let agent_lbl = status
                .agent_label
                .clone()
                .unwrap_or_else(|| "none".to_string());
            Line::from(vec![
                Span::styled("Terminal: ", Style::default().fg(p.overlay0)),
                Span::styled(
                    "[Active] ",
                    Style::default().fg(p.green).add_modifier(Modifier::BOLD),
                ),
                Span::styled(agent_lbl, Style::default().fg(p.text)),
            ])
        } else {
            Line::from(vec![
                Span::styled("Terminal: ", Style::default().fg(p.overlay0)),
                Span::styled(
                    "[Closed]",
                    Style::default().fg(p.red).add_modifier(Modifier::BOLD),
                ),
            ])
        }
    } else {
        Line::from(vec![
            Span::styled("Terminal: ", Style::default().fg(p.overlay0)),
            Span::styled("None", Style::default().fg(p.overlay1)),
        ])
    };
    frame.render_widget(Paragraph::new(terminal_line), rows[5]);

    let doc = build_kanban_description_doc(item, p);
    let cell_size = if app.kitty_graphics_enabled {
        crate::kitty_graphics::HostCellSize::from_terminal(area)
    } else {
        crate::kitty_graphics::HostCellSize::default()
    };

    let preview = super::MarkdownPreview::build(super::MarkdownPreviewRequest {
        document: &doc,
        area: rows[6],
        scroll_y: app.extensions.kanban.detail_scroll,
        scroll_x: app.extensions.kanban.detail_horizontal_scroll,
        cell_size,
        text_color: app.palette.text,
        scrollbars: super::MarkdownPreviewScrollbars::BOTH,
    });

    let desc_text = Paragraph::new(preview.lines().to_vec()).scroll((preview.scroll_y, 0));
    frame.render_widget(desc_text, preview.text_area);

    if app.kitty_graphics_enabled && cell_size.is_known() {
        if let Ok(mut placements) = app.extensions.static_image_placements.lock() {
            preview.push_image_placements(&mut placements);
        }
    }

    if let Some(scrollbar) = preview.vertical_scrollbar {
        super::render_scrollbar(
            frame,
            scrollbar.metrics,
            scrollbar.track,
            p.overlay0,
            p.overlay1,
            "▐",
        );
    }
    if let Some(scrollbar) = preview.horizontal_scrollbar {
        let track_width = scrollbar.track.width;
        let thumb_width = (((track_width as f32) * (track_width as f32))
            / (scrollbar.content_width as f32))
            .clamp(1.0, track_width as f32) as u16;
        let scrollable_width = track_width.saturating_sub(thumb_width);
        let thumb_x = if scrollbar.max_scroll_x > 0 {
            ((scrollbar.scroll_x as f32 / scrollbar.max_scroll_x as f32)
                * (scrollable_width as f32)) as u16
        } else {
            0
        };
        for x in 0..track_width {
            let is_thumb = x >= thumb_x && x < thumb_x + thumb_width;
            let symbol = if is_thumb { "━" } else { "─" };
            let style = if is_thumb {
                Style::default().fg(p.overlay1)
            } else {
                Style::default().fg(p.overlay0)
            };
            frame.render_widget(
                Paragraph::new(symbol).style(style),
                Rect::new(scrollbar.track.x + x, scrollbar.track.y, 1, 1),
            );
        }
    }

    // Footer hint
    let footer_hints = Line::from(vec![
        Span::styled(
            " [Esc] ",
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Close Details  ", Style::default().fg(p.overlay1)),
        Span::styled(
            " [c] ",
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Copy UUID  ", Style::default().fg(p.overlay1)),
        Span::styled(
            " [d] ",
            Style::default().fg(p.red).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Delete Card", Style::default().fg(p.overlay1)),
    ]);
    frame.render_widget(
        Paragraph::new(footer_hints).alignment(Alignment::Center),
        rows[7],
    );
}

pub(crate) fn get_description_text(path_str: &str) -> (String, bool) {
    if path_str.is_empty() {
        ("No description provided.".to_string(), false)
    } else {
        match std::fs::read_to_string(path_str) {
            Ok(content) => (content, false),
            Err(_) => ("NO DESCRIPTION FOUND".to_string(), true),
        }
    }
}

fn build_kanban_description_doc(
    item: &crate::api::schema::KanbanItem,
    p: &crate::app::state::Palette,
) -> super::MarkdownDocument {
    let (display_desc, is_error) = get_description_text(&item.description);
    let mut doc = super::MarkdownDocument::new();
    doc.append_text_line(
        "Description:",
        Style::default().fg(p.overlay1).add_modifier(Modifier::BOLD),
    );
    if !item.description.is_empty() {
        doc.append_link_line(
            &item.description,
            &item.description,
            Style::default()
                .fg(p.blue)
                .add_modifier(Modifier::UNDERLINED),
        );
        doc.append_empty_line();
    }
    if is_error {
        doc.append_text_line(
            &display_desc,
            Style::default().fg(p.red).add_modifier(Modifier::BOLD),
        );
    } else {
        doc.append_markdown(&display_desc, p);
    }
    doc
}

pub(crate) fn format_kanban_title(title: &str, width: u16) -> String {
    use unicode_width::UnicodeWidthChar;
    if width == 0 {
        return String::new();
    }
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for c in title.chars() {
        if c == '\n' || c == '\r' {
            continue;
        }
        let w = c.width().unwrap_or(0) as u16;
        if current_width + w > width {
            lines.push(current_line.clone());
            current_line.clear();
            current_width = 0;
            if lines.len() == 2 {
                break;
            }
        }
        current_line.push(c);
        current_width += w;
    }
    if lines.len() < 2 && !current_line.is_empty() {
        lines.push(current_line);
    }
    lines.join("\n")
}

pub fn kanban_detail_modal_size(area: Rect) -> (u16, u16) {
    let width = 80.max(area.width.saturating_mul(8) / 10);
    let height = 20.max(area.height.saturating_mul(8) / 10);
    (width, height)
}

pub fn kanban_detail_max_scroll(app: &AppState) -> u16 {
    kanban_detail_preview(app)
        .map(|preview| preview.max_scroll_y)
        .unwrap_or(0)
}

pub fn kanban_detail_max_horizontal_scroll(app: &AppState) -> u16 {
    kanban_detail_preview(app)
        .map(|preview| preview.max_scroll_x)
        .unwrap_or(0)
}

pub fn active_kanban_detail_hyperlinks(app: &AppState) -> Vec<((u16, u16), String, String)> {
    kanban_detail_preview(app)
        .map(|preview| preview.active_hyperlinks())
        .unwrap_or_default()
}

fn kanban_detail_preview(app: &AppState) -> Option<super::MarkdownPreview> {
    let uuid = app.extensions.kanban.detail_uuid.as_ref()?;
    let item = app
        .extensions
        .kanban
        .items
        .iter()
        .find(|it| it.uuid == *uuid)?;
    let desc_area = kanban_detail_description_area(app)?;

    let doc = build_kanban_description_doc(item, &app.palette);
    let cell_size = if app.kitty_graphics_enabled {
        crate::kitty_graphics::HostCellSize::from_terminal(app.view.terminal_area)
    } else {
        crate::kitty_graphics::HostCellSize::default()
    };

    Some(super::MarkdownPreview::build(
        super::MarkdownPreviewRequest {
            document: &doc,
            area: desc_area,
            scroll_y: app.extensions.kanban.detail_scroll,
            scroll_x: app.extensions.kanban.detail_horizontal_scroll,
            cell_size,
            text_color: app.palette.text,
            scrollbars: super::MarkdownPreviewScrollbars::BOTH,
        },
    ))
}

fn kanban_detail_description_area(app: &AppState) -> Option<Rect> {
    let (width, height) = kanban_detail_modal_size(app.view.terminal_area);
    let popup = centered_popup_rect(app.view.terminal_area, width, height)?;
    let inner = Block::default().borders(Borders::ALL).inner(popup);
    if inner.height < 7 || inner.width < 4 {
        return None;
    }
    let rows = Layout::vertical([
        Constraint::Length(1), // Header title
        Constraint::Length(1), // Divider
        Constraint::Length(1), // Card Title
        Constraint::Length(1), // Status Badge
        Constraint::Length(1), // UUID
        Constraint::Length(1), // Associated Terminal
        Constraint::Min(1),    // Description
        Constraint::Length(1), // Footer hint
    ])
    .split(inner);
    Some(rows[6])
}

pub fn kanban_item_at(
    app: &AppState,
    col_x: u16,
    row_y: u16,
) -> Option<(usize, usize, crate::api::schema::KanbanItem)> {
    app.extensions
        .kanban
        .board_projection(app.view.terminal_area, kanban_board_layout(app))
        .card_at(col_x, row_y)
        .map(|card| {
            (
                crate::kanban::column_index_for_status(card.item.status),
                card.row_index,
                card.item.clone(),
            )
        })
}
