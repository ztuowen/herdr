use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use super::widgets::{centered_popup_rect, render_panel_shell};
use crate::app::AppState;

pub(super) fn render_kanban(app: &AppState, frame: &mut Frame, area: Rect) {
    let p = &app.palette;

    let is_portrait = app.view.layout == crate::app::state::ViewLayout::Mobile;

    // Split main area into 4 columns/rows
    let sections = if is_portrait {
        Layout::vertical([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area)
    } else {
        Layout::horizontal([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area)
    };

    let statuses = [
        ("todo", p.overlay1, 0),
        ("in progress", p.yellow, 1),
        ("need review", p.peach, 2),
        ("done", p.green, 3),
    ];

    for (name, color, col_idx) in statuses {
        let col_area = sections[col_idx];
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
            if !is_portrait {
                let card_height: u16 = 4; // Title (up to 2 lines) + borders
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

                    let mut has_active_terminal = false;
                    if let Some(ref tid) = item.terminal_id {
                        if app.kanban_item_pane_status(tid).exists {
                            has_active_terminal = true;
                        }
                    }

                    let card_border_color = if has_active_terminal {
                        p.green
                    } else {
                        p.surface0
                    };
                    let card_bg = if is_card_selected {
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

                    let card_inner = card_block.inner(card_area);
                    frame.render_widget(card_block, card_area);

                    if card_inner.height > 0 {
                        let mut card_title_style = Style::default().add_modifier(Modifier::BOLD);
                        if has_active_terminal {
                            card_title_style =
                                card_title_style.fg(p.green).add_modifier(Modifier::ITALIC);
                        } else {
                            card_title_style = card_title_style.fg(p.text);
                        }

                        let formatted_title =
                            format_kanban_title(item.title.as_str(), card_inner.width);
                        frame.render_widget(
                            Paragraph::new(formatted_title).style(card_title_style),
                            card_inner,
                        );
                    }
                }
            } else {
                // Portrait mode: horizontal cards inside status rows, spanning full width
                let card_width = inner_area.width;
                let max_visible_cards = 1;

                // Compute scroll offset for the focused column (now row)
                let scroll_offset = if is_col_focused {
                    app.kanban_selected_row
                } else {
                    0
                };

                let visible_items = items.iter().skip(scroll_offset).take(max_visible_cards);
                for (idx, item) in visible_items.enumerate() {
                    let actual_idx = scroll_offset + idx;
                    let card_x = inner_area.x;

                    let card_height = 4u16.min(inner_area.height);
                    let card_area = Rect::new(card_x, inner_area.y, card_width, card_height);

                    let is_card_selected = is_col_focused && app.kanban_selected_row == actual_idx;

                    let mut has_active_terminal = false;
                    if let Some(ref tid) = item.terminal_id {
                        if app.kanban_item_pane_status(tid).exists {
                            has_active_terminal = true;
                        }
                    }

                    let card_border_color = if has_active_terminal {
                        p.green
                    } else {
                        p.surface0
                    };
                    let card_bg = if is_card_selected {
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

                    let card_inner = card_block.inner(card_area);
                    frame.render_widget(card_block, card_area);

                    if card_inner.height > 0 {
                        let mut card_title_style = Style::default().add_modifier(Modifier::BOLD);
                        if has_active_terminal {
                            card_title_style =
                                card_title_style.fg(p.green).add_modifier(Modifier::ITALIC);
                        } else {
                            card_title_style = card_title_style.fg(p.text);
                        }

                        let formatted_title =
                            format_kanban_title(item.title.as_str(), card_inner.width);
                        frame.render_widget(
                            Paragraph::new(formatted_title).style(card_title_style),
                            card_inner,
                        );
                    }
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
            render_kanban_detail_modal(app, frame, area, item);
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
    let (width, height) = app.kanban_detail_modal_size();
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

    // UUID
    let uuid_line = Line::from(vec![
        Span::styled("UUID: ", Style::default().fg(p.overlay0)),
        Span::styled(item.uuid.as_str(), Style::default().fg(p.text)),
    ]);
    frame.render_widget(Paragraph::new(uuid_line), rows[4]);

    // Associated Terminal / Pane status
    let terminal_line = if let Some(ref tid) = item.terminal_id {
        let status = app.kanban_item_pane_status(tid);
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

    // Description block
    let desc_title = Span::styled(
        "Description:",
        Style::default().fg(p.overlay1).add_modifier(Modifier::BOLD),
    );
    let (display_desc, is_error) = get_description_text(&item.description);

    let max_scroll = app.kanban_detail_max_scroll();
    let metrics = crate::pane::ScrollMetrics {
        offset_from_bottom: max_scroll.saturating_sub(app.kanban_detail_scroll) as usize,
        max_offset_from_bottom: max_scroll as usize,
        viewport_rows: rows[6].height.max(1) as usize,
    };
    let track = super::release_notes_scrollbar_rect(rows[6], metrics);
    let desc_area = track
        .map(|_| {
            Rect::new(
                rows[6].x,
                rows[6].y,
                rows[6].width.saturating_sub(1),
                rows[6].height,
            )
        })
        .unwrap_or(rows[6]);

    let mut all_md_lines = Vec::new();
    all_md_lines.push(super::MarkdownLine {
        spans: vec![super::MarkdownSpan::Text(desc_title)],
        is_code_block: false,
    });
    if !item.description.is_empty() {
        all_md_lines.push(super::MarkdownLine {
            spans: vec![super::MarkdownSpan::Text(Span::styled(
                item.description.clone(),
                Style::default().fg(p.overlay0),
            ))],
            is_code_block: false,
        });
        all_md_lines.push(super::MarkdownLine {
            spans: vec![super::MarkdownSpan::Text(Span::raw(""))],
            is_code_block: false,
        });
    }
    if is_error {
        all_md_lines.push(super::MarkdownLine {
            spans: vec![super::MarkdownSpan::Text(Span::styled(
                display_desc,
                Style::default().fg(p.red).add_modifier(Modifier::BOLD),
            ))],
            is_code_block: false,
        });
    } else {
        all_md_lines.extend(super::parse_markdown_with_links(&display_desc, p));
    }

    let wrapped = super::wrap_markdown(&all_md_lines, desc_area.width as usize);

    let desc_text = Paragraph::new(wrapped.lines).scroll((app.kanban_detail_scroll, 0));
    frame.render_widget(desc_text, desc_area);

    if let Some(track) = track {
        super::render_scrollbar(frame, metrics, track, p.overlay0, p.overlay1, "▐");
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
        Paragraph::new(footer_hints).alignment(ratatui::layout::Alignment::Center),
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
