use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

use super::widgets::{centered_popup_rect, render_panel_shell};
use crate::app::state::Palette;
use crate::app::AppState;

pub(super) fn render_kanban(app: &AppState, frame: &mut Frame, area: Rect) {
    let p = &app.palette;

    // Split main area into 4 columns
    let cols = Layout::horizontal([
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
    ])
    .split(area);

    let statuses = [
        ("todo", p.overlay1, 0),
        ("in progress", p.yellow, 1),
        ("need review", p.peach, 2),
        ("done", p.green, 3),
    ];

    for (name, color, col_idx) in statuses {
        let col_area = cols[col_idx];
        let items = app.kanban_items_in_column(col_idx);
        let count = items.len();

        let is_col_focused = app.kanban_selected_col == col_idx;

        // Render column block
        let col_border_color = if is_col_focused { p.accent } else { p.surface0 };
        let col_title = format!(" {} ({}) ", name.to_uppercase(), count);
        let col_block = Block::default()
            .borders(Borders::ALL)
            .border_type(if is_col_focused {
                BorderType::Thick
            } else {
                BorderType::Plain
            })
            .border_style(Style::default().fg(col_border_color))
            .title(Span::styled(
                col_title,
                Style::default()
                    .fg(if is_col_focused { p.accent } else { color })
                    .add_modifier(Modifier::BOLD),
            ));

        let inner_area = col_block.inner(col_area);
        frame.render_widget(col_block, col_area);

        // Draw cards in this column
        if count > 0 {
            let card_height: u16 = 4; // Title + description + borders
            let spacing: u16 = 1;
            let total_card_height = card_height + spacing;
            let max_visible_cards = inner_area
                .height
                .checked_div(total_card_height)
                .map(usize::from)
                .unwrap_or(0);

            // Compute scroll offset for the focused column
            let scroll_offset = if is_col_focused {
                let row = app.kanban_selected_row;
                if max_visible_cards == 0 {
                    0
                } else if row >= max_visible_cards {
                    row - max_visible_cards + 1
                } else {
                    0
                }
            } else {
                0
            };

            let visible_items = items.iter().skip(scroll_offset).take(max_visible_cards);
            for (idx, item) in visible_items.enumerate() {
                let actual_idx = scroll_offset + idx;
                let card_y = inner_area.y + (idx as u16 * total_card_height);
                if card_y + card_height > inner_area.y + inner_area.height {
                    break;
                }

                let card_area = Rect::new(inner_area.x, card_y, inner_area.width, card_height);

                let is_card_selected = is_col_focused && app.kanban_selected_row == actual_idx;
                let card_border_color = if is_card_selected {
                    p.accent
                } else {
                    p.surface0
                };
                let card_bg = if is_card_selected {
                    p.surface0
                } else {
                    p.surface_dim
                };

                let card_block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(if is_card_selected {
                        BorderType::Double
                    } else {
                        BorderType::Plain
                    })
                    .border_style(Style::default().fg(card_border_color))
                    .style(Style::default().bg(card_bg));

                let card_inner = card_block.inner(card_area);
                frame.render_widget(card_block, card_area);

                if card_inner.height > 0 {
                    let card_title_style = Style::default().fg(p.text).add_modifier(Modifier::BOLD);
                    frame.render_widget(
                        Paragraph::new(item.title.as_str())
                            .style(card_title_style)
                            .wrap(Wrap { trim: true }),
                        Rect::new(card_inner.x, card_inner.y, card_inner.width, 1),
                    );
                }
                if card_inner.height > 1 {
                    let desc_style = Style::default().fg(p.overlay0);
                    let display_desc = if item.description.is_empty() {
                        "No description."
                    } else {
                        item.description.as_str()
                    };
                    frame.render_widget(
                        Paragraph::new(display_desc)
                            .style(desc_style)
                            .wrap(Wrap { trim: true }),
                        Rect::new(
                            card_inner.x,
                            card_inner.y + 1,
                            card_inner.width,
                            card_inner.height - 1,
                        ),
                    );
                }
            }
        } else {
            // Draw empty message
            let empty_text = Paragraph::new("No items")
                .style(Style::default().fg(p.overlay0))
                .alignment(ratatui::layout::Alignment::Center);
            let empty_rect = Rect::new(
                inner_area.x,
                inner_area.y + inner_area.height / 2,
                inner_area.width,
                1,
            );
            frame.render_widget(empty_text, empty_rect);
        }
    }

    // Render detailed modal if app.kanban_detail_uuid is Some
    if let Some(ref uuid) = app.kanban_detail_uuid {
        if let Some(item) = app.kanban_items.iter().find(|it| it.uuid == *uuid) {
            render_kanban_detail_modal(frame, area, item, p);
        }
    }
}

fn render_kanban_detail_modal(
    frame: &mut Frame,
    area: Rect,
    item: &crate::api::schema::KanbanItem,
    p: &Palette,
) {
    let Some(popup) = centered_popup_rect(area, 64, 14) else {
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
    let status_color = match item.status {
        crate::api::schema::KanbanStatus::Todo => p.overlay1,
        crate::api::schema::KanbanStatus::InProgress => p.yellow,
        crate::api::schema::KanbanStatus::NeedReview => p.peach,
        crate::api::schema::KanbanStatus::Done => p.green,
    };
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

    // Description block
    let desc_title = Span::styled(
        "Description:\n",
        Style::default().fg(p.overlay1).add_modifier(Modifier::BOLD),
    );
    let display_desc = if item.description.is_empty() {
        "No description provided."
    } else {
        item.description.as_str()
    };
    let desc_text = Paragraph::new(vec![
        Line::from(desc_title),
        Line::from(Span::styled(display_desc, Style::default().fg(p.text))),
    ])
    .wrap(Wrap { trim: true });
    frame.render_widget(desc_text, rows[4]);

    // Footer hint
    let footer_hints = Line::from(vec![
        Span::styled(
            " [Esc] ",
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Close Details  ", Style::default().fg(p.overlay1)),
        Span::styled(
            " [d] ",
            Style::default().fg(p.red).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Delete Card", Style::default().fg(p.overlay1)),
    ]);
    frame.render_widget(
        Paragraph::new(footer_hints).alignment(ratatui::layout::Alignment::Center),
        rows[5],
    );
}
