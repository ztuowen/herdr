use once_cell::sync::Lazy;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

use crate::app::state::Palette;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

fn is_light_palette(palette: &Palette) -> bool {
    if let ratatui::style::Color::Rgb(r, g, b) = palette.panel_bg {
        let luminance = 0.299 * (r as f32) + 0.587 * (g as f32) + 0.114 * (b as f32);
        luminance > 128.0
    } else {
        false
    }
}

fn map_syntect_style(syntect_style: syntect::highlighting::Style, palette: &Palette) -> Style {
    let mut style = Style::default().bg(palette.surface1);

    let fg = syntect_style.foreground;
    style = style.fg(ratatui::style::Color::Rgb(fg.r, fg.g, fg.b));

    let font = syntect_style.font_style;
    if font.contains(syntect::highlighting::FontStyle::BOLD) {
        style = style.add_modifier(Modifier::BOLD);
    }
    if font.contains(syntect::highlighting::FontStyle::ITALIC) {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if font.contains(syntect::highlighting::FontStyle::UNDERLINE) {
        style = style.add_modifier(Modifier::UNDERLINED);
    }

    style
}

fn highlight_code_block(lines: &[String], lang: &str, palette: &Palette) -> Vec<MarkdownLine> {
    let syntax = SYNTAX_SET
        .find_syntax_by_token(lang)
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    let theme_name = if is_light_palette(palette) {
        "base16-ocean.light"
    } else {
        "base16-ocean.dark"
    };

    let theme = &THEME_SET.themes[theme_name];
    let mut h = HighlightLines::new(syntax, theme);

    let mut out = Vec::new();

    for line in lines {
        let mut spans = vec![MarkdownSpan::Text(Span::styled(
            "▏",
            Style::default().fg(palette.accent).bg(palette.surface1),
        ))];

        let line_with_nl = format!("{}\n", line);
        let styled_ops = h
            .highlight_line(&line_with_nl, &SYNTAX_SET)
            .unwrap_or_default();
        for (syntect_style, text) in styled_ops {
            let text_trimmed = text.strip_suffix('\n').unwrap_or(text);
            if !text_trimmed.is_empty() {
                let style = map_syntect_style(syntect_style, palette);
                spans.push(MarkdownSpan::Text(Span::styled(
                    text_trimmed.to_string(),
                    style,
                )));
            }
        }

        out.push(MarkdownLine {
            spans,
            is_code_block: true,
            is_blockquote: false,
            is_table_row: false,
        });
    }

    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DelimiterKind {
    Code,
    BoldAsterisk,
    BoldUnderscore,
    ItalicAsterisk,
    ItalicUnderscore,
    Link {
        label_end: usize,
        url_start: usize,
        url_end: usize,
    },
}

struct DelimiterMatch {
    kind: DelimiterKind,
    start_idx: usize,
    end_idx: usize, // index after the closing delimiter
}

fn find_first_match(chars: &[char]) -> Option<DelimiterMatch> {
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '`' {
            // Look for closing '`'
            let mut j = i + 1;
            while j < chars.len() {
                if chars[j] == '`' {
                    return Some(DelimiterMatch {
                        kind: DelimiterKind::Code,
                        start_idx: i,
                        end_idx: j + 1,
                    });
                }
                j += 1;
            }
        } else if chars[i] == '*' && chars.get(i + 1) == Some(&'*') {
            // Look for closing '**'
            let mut j = i + 2;
            while j < chars.len() {
                if chars[j] == '*' && chars.get(j + 1) == Some(&'*') {
                    return Some(DelimiterMatch {
                        kind: DelimiterKind::BoldAsterisk,
                        start_idx: i,
                        end_idx: j + 2,
                    });
                }
                j += 1;
            }
        } else if chars[i] == '_' && chars.get(i + 1) == Some(&'_') {
            // Look for closing '__'
            let mut j = i + 2;
            while j < chars.len() {
                if chars[j] == '_' && chars.get(j + 1) == Some(&'_') {
                    return Some(DelimiterMatch {
                        kind: DelimiterKind::BoldUnderscore,
                        start_idx: i,
                        end_idx: j + 2,
                    });
                }
                j += 1;
            }
        } else if chars[i] == '*' {
            // Look for closing '*' (skipping '**')
            let mut j = i + 1;
            while j < chars.len() {
                if chars[j] == '*' {
                    if chars.get(j + 1) == Some(&'*') {
                        // Skip double asterisk
                        j += 2;
                    } else {
                        return Some(DelimiterMatch {
                            kind: DelimiterKind::ItalicAsterisk,
                            start_idx: i,
                            end_idx: j + 1,
                        });
                    }
                } else {
                    j += 1;
                }
            }
        } else if chars[i] == '_' {
            // Look for closing '_' (skipping '__')
            let mut j = i + 1;
            while j < chars.len() {
                if chars[j] == '_' {
                    if chars.get(j + 1) == Some(&'_') {
                        // Skip double underscore
                        j += 2;
                    } else {
                        return Some(DelimiterMatch {
                            kind: DelimiterKind::ItalicUnderscore,
                            start_idx: i,
                            end_idx: j + 1,
                        });
                    }
                } else {
                    j += 1;
                }
            }
        } else if chars[i] == '[' {
            // Look for matching link
            let mut j = i + 1;
            while j < chars.len() {
                if chars[j] == ']' && chars.get(j + 1) == Some(&'(') {
                    let mut k = j + 2;
                    while k < chars.len() {
                        if chars[k] == ')' {
                            return Some(DelimiterMatch {
                                kind: DelimiterKind::Link {
                                    label_end: j,
                                    url_start: j + 2,
                                    url_end: k,
                                },
                                start_idx: i,
                                end_idx: k + 1,
                            });
                        }
                        k += 1;
                    }
                }
                j += 1;
            }
        }
        i += 1;
    }
    None
}

fn parse_inline_chars(
    chars: &[char],
    palette: &Palette,
    current_style: Style,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut start = 0;

    while start < chars.len() {
        if let Some(m) = find_first_match(&chars[start..]) {
            let match_start = start + m.start_idx;
            let match_end = start + m.end_idx;

            if match_start > start {
                let plain_text: String = chars[start..match_start].iter().collect();
                spans.push(Span::styled(plain_text, current_style));
            }

            match m.kind {
                DelimiterKind::Code => {
                    let code_content: String =
                        chars[match_start + 1..match_end - 1].iter().collect();
                    spans.push(Span::styled(
                        code_content,
                        Style::default()
                            .fg(palette.accent)
                            .bg(palette.surface0)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                DelimiterKind::BoldAsterisk | DelimiterKind::BoldUnderscore => {
                    let content_chars = &chars[match_start + 2..match_end - 2];
                    let bold_style = current_style.add_modifier(Modifier::BOLD);
                    spans.extend(parse_inline_chars(content_chars, palette, bold_style));
                }
                DelimiterKind::ItalicAsterisk | DelimiterKind::ItalicUnderscore => {
                    let content_chars = &chars[match_start + 1..match_end - 1];
                    let italic_style = current_style.add_modifier(Modifier::ITALIC);
                    spans.extend(parse_inline_chars(content_chars, palette, italic_style));
                }
                DelimiterKind::Link {
                    label_end,
                    url_start: _,
                    url_end: _,
                } => {
                    let abs_label_end = start + label_end;
                    let label_chars = &chars[match_start + 1..abs_label_end];
                    let link_style = Style::default()
                        .fg(palette.blue)
                        .add_modifier(Modifier::UNDERLINED);
                    spans.extend(parse_inline_chars(label_chars, palette, link_style));
                }
            }

            start = match_end;
        } else {
            let plain_text: String = chars[start..].iter().collect();
            spans.push(Span::styled(plain_text, current_style));
            break;
        }
    }
    spans
}

// Allowed because this helper is used in tests and kept for backward compatibility.
#[allow(dead_code)]
pub(crate) fn parse_inline_style(
    text: &str,
    palette: &Palette,
    current_style: Style,
) -> Vec<Span<'static>> {
    let chars: Vec<char> = text.chars().collect();
    parse_inline_chars(&chars, palette, current_style)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkdownSpan {
    Text(Span<'static>),
    Link {
        label_spans: Vec<Span<'static>>,
        url: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownLine {
    pub spans: Vec<MarkdownSpan>,
    pub is_code_block: bool,
    pub is_blockquote: bool,
    pub is_table_row: bool,
}

fn parse_inline_chars_with_links(
    chars: &[char],
    palette: &Palette,
    current_style: Style,
) -> Vec<MarkdownSpan> {
    let mut spans = Vec::new();
    let mut start = 0;

    while start < chars.len() {
        if let Some(m) = find_first_match(&chars[start..]) {
            let match_start = start + m.start_idx;
            let match_end = start + m.end_idx;

            if match_start > start {
                let plain_text: String = chars[start..match_start].iter().collect();
                spans.push(MarkdownSpan::Text(Span::styled(plain_text, current_style)));
            }

            match m.kind {
                DelimiterKind::Code => {
                    let code_content: String =
                        chars[match_start + 1..match_end - 1].iter().collect();
                    spans.push(MarkdownSpan::Text(Span::styled(
                        code_content,
                        Style::default()
                            .fg(palette.accent)
                            .bg(palette.surface0)
                            .add_modifier(Modifier::BOLD),
                    )));
                }
                DelimiterKind::BoldAsterisk | DelimiterKind::BoldUnderscore => {
                    let content_chars = &chars[match_start + 2..match_end - 2];
                    let bold_style = current_style.add_modifier(Modifier::BOLD);
                    spans.extend(parse_inline_chars_with_links(
                        content_chars,
                        palette,
                        bold_style,
                    ));
                }
                DelimiterKind::ItalicAsterisk | DelimiterKind::ItalicUnderscore => {
                    let content_chars = &chars[match_start + 1..match_end - 1];
                    let italic_style = current_style.add_modifier(Modifier::ITALIC);
                    spans.extend(parse_inline_chars_with_links(
                        content_chars,
                        palette,
                        italic_style,
                    ));
                }
                DelimiterKind::Link {
                    label_end,
                    url_start,
                    url_end,
                } => {
                    let abs_label_end = start + label_end;
                    let label_chars = &chars[match_start + 1..abs_label_end];

                    let abs_url_start = start + url_start;
                    let abs_url_end = start + url_end;
                    let url_str: String = chars[abs_url_start..abs_url_end].iter().collect();

                    let link_style = Style::default()
                        .fg(palette.blue)
                        .add_modifier(Modifier::UNDERLINED);
                    let label_spans = parse_inline_chars(label_chars, palette, link_style);

                    spans.push(MarkdownSpan::Link {
                        label_spans,
                        url: url_str,
                    });
                }
            }

            start = match_end;
        } else {
            let plain_text: String = chars[start..].iter().collect();
            spans.push(MarkdownSpan::Text(Span::styled(plain_text, current_style)));
            break;
        }
    }
    spans
}

pub(crate) fn parse_inline_style_with_links(
    text: &str,
    palette: &Palette,
    current_style: Style,
) -> Vec<MarkdownSpan> {
    let chars: Vec<char> = text.chars().collect();
    parse_inline_chars_with_links(&chars, palette, current_style)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TableAlign {
    Left,
    Center,
    Right,
}

fn is_delimiter_row(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.contains('|') {
        return false;
    }
    let parts: Vec<&str> = trimmed.split('|').collect();
    let start_idx = if parts[0].trim().is_empty() && parts.len() > 1 {
        1
    } else {
        0
    };
    let end_idx = if parts[parts.len() - 1].trim().is_empty() && parts.len() > 1 {
        parts.len() - 1
    } else {
        parts.len()
    };

    if start_idx >= end_idx {
        return false;
    }

    for part in &parts[start_idx..end_idx] {
        let p = part.trim();
        if p.is_empty() {
            return false;
        }
        if !p.chars().all(|c| c == '-' || c == ':' || c.is_whitespace()) {
            return false;
        }
        if !p.contains('-') {
            return false;
        }
    }
    true
}

fn split_table_row(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'|') {
            current.push('|');
            chars.next(); // consume '|'
        } else if c == '|' {
            cells.push(current);
            current = String::new();
        } else {
            current.push(c);
        }
    }
    cells.push(current);

    let mut start_idx = 0;
    let mut end_idx = cells.len();
    if cells.len() > 1 && cells[0].trim().is_empty() && line.trim().starts_with('|') {
        start_idx = 1;
    }
    if cells.len() > start_idx + 1
        && cells[cells.len() - 1].trim().is_empty()
        && line.trim().ends_with('|')
    {
        end_idx = cells.len() - 1;
    }

    cells[start_idx..end_idx]
        .iter()
        .map(|s| s.trim().to_string())
        .collect()
}

fn spans_width(spans: &[MarkdownSpan]) -> usize {
    let mut w = 0;
    for span in spans {
        match span {
            MarkdownSpan::Text(s) => {
                w += unicode_width::UnicodeWidthStr::width(s.content.as_ref());
            }
            MarkdownSpan::Link { label_spans, .. } => {
                for s in label_spans {
                    w += unicode_width::UnicodeWidthStr::width(s.content.as_ref());
                }
            }
        }
    }
    w
}

fn pad_spans(
    mut spans: Vec<MarkdownSpan>,
    target_width: usize,
    align: TableAlign,
    pad_style: Style,
) -> Vec<MarkdownSpan> {
    let current_w = spans_width(&spans);
    if current_w >= target_width {
        return spans;
    }
    let total_pad = target_width - current_w;
    let (left_pad, right_pad) = match align {
        TableAlign::Left => (0, total_pad),
        TableAlign::Right => (total_pad, 0),
        TableAlign::Center => (total_pad / 2, total_pad - (total_pad / 2)),
    };

    if left_pad > 0 {
        let left_space = Span::styled(" ".repeat(left_pad), pad_style);
        spans.insert(0, MarkdownSpan::Text(left_space));
    }
    if right_pad > 0 {
        let right_space = Span::styled(" ".repeat(right_pad), pad_style);
        spans.push(MarkdownSpan::Text(right_space));
    }
    spans
}

fn parse_alignment(cell: &str) -> TableAlign {
    let trimmed = cell.trim();
    let starts = trimmed.starts_with(':');
    let ends = trimmed.ends_with(':');
    if starts && ends {
        TableAlign::Center
    } else if ends {
        TableAlign::Right
    } else {
        TableAlign::Left
    }
}

// Allowed because col_idx is used to index multiple aligned vectors (alignments, col_widths, header_spans, body_spans)
// and manual index-based loops are clearer and safer than chained iterators here.
#[allow(clippy::needless_range_loop)]
fn parse_and_format_table(
    header_raw: &str,
    delimiter_raw: &str,
    body_raws: &[&str],
    palette: &Palette,
) -> Vec<MarkdownLine> {
    let header_cells = split_table_row(header_raw);
    let delimiter_cells = split_table_row(delimiter_raw);
    let num_cols = delimiter_cells.len();
    if num_cols == 0 {
        return Vec::new();
    }

    let alignments: Vec<TableAlign> = delimiter_cells.iter().map(|c| parse_alignment(c)).collect();

    let mut body_rows = Vec::new();
    for body_raw in body_raws {
        let mut cells = split_table_row(body_raw);
        if cells.len() > num_cols {
            cells.truncate(num_cols);
        } else {
            while cells.len() < num_cols {
                cells.push(String::new());
            }
        }
        body_rows.push(cells);
    }

    let header_style = Style::default()
        .fg(palette.teal)
        .add_modifier(Modifier::BOLD);
    let body_style = Style::default().fg(palette.text);

    let mut header_spans: Vec<Vec<MarkdownSpan>> = header_cells
        .iter()
        .take(num_cols)
        .map(|cell| parse_inline_style_with_links(cell, palette, header_style))
        .collect();
    while header_spans.len() < num_cols {
        header_spans.push(Vec::new());
    }

    let body_spans: Vec<Vec<Vec<MarkdownSpan>>> = body_rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| parse_inline_style_with_links(cell, palette, body_style))
                .collect()
        })
        .collect();

    let mut col_widths = vec![3; num_cols];
    for col_idx in 0..num_cols {
        let mut max_w = spans_width(&header_spans[col_idx]);
        for row in &body_spans {
            let cell_w = spans_width(&row[col_idx]);
            if cell_w > max_w {
                max_w = cell_w;
            }
        }
        col_widths[col_idx] = col_widths[col_idx].max(max_w);
    }

    let mut lines = Vec::new();
    let border_style = Style::default().fg(palette.surface1);

    let mut top_spans = Vec::new();
    top_spans.push(MarkdownSpan::Text(Span::styled("┌", border_style)));
    for col_idx in 0..num_cols {
        let dashes = "─".repeat(col_widths[col_idx] + 2);
        top_spans.push(MarkdownSpan::Text(Span::styled(dashes, border_style)));
        if col_idx < num_cols - 1 {
            top_spans.push(MarkdownSpan::Text(Span::styled("┬", border_style)));
        }
    }
    top_spans.push(MarkdownSpan::Text(Span::styled("┐", border_style)));
    lines.push(MarkdownLine {
        spans: top_spans,
        is_code_block: false,
        is_blockquote: false,
        is_table_row: true,
    });

    let mut h_spans = Vec::new();
    h_spans.push(MarkdownSpan::Text(Span::styled("│", border_style)));
    for col_idx in 0..num_cols {
        let align = alignments[col_idx];
        let cell_spans = pad_spans(
            header_spans[col_idx].clone(),
            col_widths[col_idx],
            align,
            header_style,
        );
        h_spans.push(MarkdownSpan::Text(Span::styled(" ", header_style)));
        h_spans.extend(cell_spans);
        h_spans.push(MarkdownSpan::Text(Span::styled(" ", header_style)));
        h_spans.push(MarkdownSpan::Text(Span::styled("│", border_style)));
    }
    lines.push(MarkdownLine {
        spans: h_spans,
        is_code_block: false,
        is_blockquote: false,
        is_table_row: true,
    });

    let mut mid_spans = Vec::new();
    mid_spans.push(MarkdownSpan::Text(Span::styled("├", border_style)));
    for col_idx in 0..num_cols {
        let dashes = "─".repeat(col_widths[col_idx] + 2);
        mid_spans.push(MarkdownSpan::Text(Span::styled(dashes, border_style)));
        if col_idx < num_cols - 1 {
            mid_spans.push(MarkdownSpan::Text(Span::styled("┼", border_style)));
        }
    }
    mid_spans.push(MarkdownSpan::Text(Span::styled("┤", border_style)));
    lines.push(MarkdownLine {
        spans: mid_spans,
        is_code_block: false,
        is_blockquote: false,
        is_table_row: true,
    });

    for row_spans in body_spans {
        let mut r_spans = Vec::new();
        r_spans.push(MarkdownSpan::Text(Span::styled("│", border_style)));
        for col_idx in 0..num_cols {
            let align = alignments[col_idx];
            let cell_spans = pad_spans(
                row_spans[col_idx].clone(),
                col_widths[col_idx],
                align,
                body_style,
            );
            r_spans.push(MarkdownSpan::Text(Span::styled(" ", body_style)));
            r_spans.extend(cell_spans);
            r_spans.push(MarkdownSpan::Text(Span::styled(" ", body_style)));
            r_spans.push(MarkdownSpan::Text(Span::styled("│", border_style)));
        }
        lines.push(MarkdownLine {
            spans: r_spans,
            is_code_block: false,
            is_blockquote: false,
            is_table_row: true,
        });
    }

    let mut bot_spans = Vec::new();
    bot_spans.push(MarkdownSpan::Text(Span::styled("└", border_style)));
    for col_idx in 0..num_cols {
        let dashes = "─".repeat(col_widths[col_idx] + 2);
        bot_spans.push(MarkdownSpan::Text(Span::styled(dashes, border_style)));
        if col_idx < num_cols - 1 {
            bot_spans.push(MarkdownSpan::Text(Span::styled("┴", border_style)));
        }
    }
    bot_spans.push(MarkdownSpan::Text(Span::styled("┘", border_style)));
    lines.push(MarkdownLine {
        spans: bot_spans,
        is_code_block: false,
        is_blockquote: false,
        is_table_row: true,
    });

    lines
}

pub fn parse_markdown_with_links(text: &str, palette: &Palette) -> Vec<MarkdownLine> {
    let mut lines = Vec::new();
    let mut in_code_block = false;
    let mut code_block_lang = String::new();
    let mut code_block_lines = Vec::new();
    let input_lines: Vec<&str> = text.lines().collect();
    let mut line_idx = 0;

    while line_idx < input_lines.len() {
        let line = input_lines[line_idx];
        let trimmed = line.trim();

        // Handle table detection
        if !in_code_block
            && line_idx + 1 < input_lines.len()
            && is_delimiter_row(input_lines[line_idx + 1])
            && line.contains('|')
        {
            let header_raw = line;
            let delimiter_raw = input_lines[line_idx + 1];
            let mut body_raws = Vec::new();
            let mut k = 2;
            while line_idx + k < input_lines.len() {
                let next_line = input_lines[line_idx + k];
                let trimmed_next = next_line.trim();
                if trimmed_next.contains('|')
                    && !trimmed_next.starts_with("```")
                    && !trimmed_next.starts_with("#")
                    && trimmed_next != "---"
                    && trimmed_next != "***"
                    && trimmed_next != "___"
                    && !trimmed_next.is_empty()
                {
                    body_raws.push(next_line);
                    k += 1;
                } else {
                    break;
                }
            }

            let table_lines =
                parse_and_format_table(header_raw, delimiter_raw, &body_raws, palette);
            lines.extend(table_lines);
            line_idx += k;
            continue;
        }

        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            if in_code_block {
                code_block_lang = trimmed
                    .strip_prefix("```")
                    .unwrap_or("")
                    .trim()
                    .to_lowercase();
            } else {
                let highlighted =
                    highlight_code_block(&code_block_lines, &code_block_lang, palette);
                lines.extend(highlighted);
                code_block_lines.clear();
                code_block_lang.clear();
            }
            line_idx += 1;
            continue;
        }

        if in_code_block {
            code_block_lines.push(line.to_string());
            line_idx += 1;
            continue;
        }

        // Check for horizontal rule
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            lines.push(MarkdownLine {
                spans: vec![MarkdownSpan::Text(Span::styled(
                    "─".repeat(40),
                    Style::default().fg(palette.surface0),
                ))],
                is_code_block: false,
                is_blockquote: false,
                is_table_row: false,
            });
            line_idx += 1;
            continue;
        }

        // Check for headers
        if let Some(content) = line.strip_prefix("# ") {
            let mut spans = vec![MarkdownSpan::Text(Span::styled(
                "█ ",
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            ))];
            spans.extend(parse_inline_style_with_links(
                content,
                palette,
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            ));
            lines.push(MarkdownLine {
                spans,
                is_code_block: false,
                is_blockquote: false,
                is_table_row: false,
            });
        } else if let Some(content) = line.strip_prefix("## ") {
            let spans = parse_inline_style_with_links(
                content,
                palette,
                Style::default()
                    .fg(palette.teal)
                    .add_modifier(Modifier::BOLD),
            );
            lines.push(MarkdownLine {
                spans,
                is_code_block: false,
                is_blockquote: false,
                is_table_row: false,
            });
        } else if let Some(content) = line.strip_prefix("### ") {
            let spans = parse_inline_style_with_links(
                content,
                palette,
                Style::default()
                    .fg(palette.peach)
                    .add_modifier(Modifier::BOLD),
            );
            lines.push(MarkdownLine {
                spans,
                is_code_block: false,
                is_blockquote: false,
                is_table_row: false,
            });
        } else if let Some(content) = line.strip_prefix("#### ") {
            let spans = parse_inline_style_with_links(
                content,
                palette,
                Style::default()
                    .fg(palette.mauve)
                    .add_modifier(Modifier::BOLD),
            );
            lines.push(MarkdownLine {
                spans,
                is_code_block: false,
                is_blockquote: false,
                is_table_row: false,
            });
        } else if line == ">" || line.starts_with("> ") {
            // Blockquote
            let content = line.strip_prefix("> ").unwrap_or("");
            let mut spans = vec![MarkdownSpan::Text(Span::styled(
                "│ ",
                Style::default().fg(palette.accent),
            ))];
            spans.extend(parse_inline_style_with_links(
                content,
                palette,
                Style::default()
                    .fg(palette.overlay1)
                    .add_modifier(Modifier::ITALIC),
            ));
            lines.push(MarkdownLine {
                spans,
                is_code_block: false,
                is_blockquote: true,
                is_table_row: false,
            });
        } else {
            // Check for lists
            let indent_len = line.chars().take_while(|&c| c == ' ').count();
            let suffix = &line[indent_len..];

            if suffix.starts_with("- ") || suffix.starts_with("* ") || suffix.starts_with("+ ") {
                let content = &suffix[2..];
                let indent_str = " ".repeat(indent_len);

                if content == "[ ]" || content.starts_with("[ ] ") {
                    let task_text = if content.len() > 4 { &content[4..] } else { "" };
                    let mut spans = vec![
                        MarkdownSpan::Text(Span::styled(indent_str, Style::default())),
                        MarkdownSpan::Text(Span::styled(
                            "[ ] ",
                            Style::default().fg(palette.overlay1),
                        )),
                    ];
                    spans.extend(parse_inline_style_with_links(
                        task_text,
                        palette,
                        Style::default().fg(palette.text),
                    ));
                    lines.push(MarkdownLine {
                        spans,
                        is_code_block: false,
                        is_blockquote: false,
                        is_table_row: false,
                    });
                } else if content == "[x]"
                    || content.starts_with("[x] ")
                    || content == "[X]"
                    || content.starts_with("[X] ")
                {
                    let task_text = if content.len() > 4 { &content[4..] } else { "" };
                    let mut spans = vec![
                        MarkdownSpan::Text(Span::styled(indent_str, Style::default())),
                        MarkdownSpan::Text(Span::styled(
                            "[✓] ",
                            Style::default().fg(palette.green),
                        )),
                    ];
                    spans.extend(parse_inline_style_with_links(
                        task_text,
                        palette,
                        Style::default()
                            .fg(palette.subtext0)
                            .add_modifier(Modifier::CROSSED_OUT),
                    ));
                    lines.push(MarkdownLine {
                        spans,
                        is_code_block: false,
                        is_blockquote: false,
                        is_table_row: false,
                    });
                } else {
                    let mut spans = vec![
                        MarkdownSpan::Text(Span::styled(indent_str, Style::default())),
                        MarkdownSpan::Text(Span::styled("• ", Style::default().fg(palette.accent))),
                    ];
                    spans.extend(parse_inline_style_with_links(
                        content,
                        palette,
                        Style::default().fg(palette.text),
                    ));
                    lines.push(MarkdownLine {
                        spans,
                        is_code_block: false,
                        is_blockquote: false,
                        is_table_row: false,
                    });
                }
            } else {
                // Check for ordered list
                let digit_chars: String =
                    suffix.chars().take_while(|c| c.is_ascii_digit()).collect();
                if !digit_chars.is_empty() && suffix[digit_chars.len()..].starts_with(". ") {
                    let num_len = digit_chars.len();
                    let content = &suffix[num_len + 2..];
                    let indent_str = " ".repeat(indent_len);
                    let mut spans = vec![
                        MarkdownSpan::Text(Span::styled(indent_str, Style::default())),
                        MarkdownSpan::Text(Span::styled(
                            format!("{}. ", digit_chars),
                            Style::default().fg(palette.accent),
                        )),
                    ];
                    spans.extend(parse_inline_style_with_links(
                        content,
                        palette,
                        Style::default().fg(palette.text),
                    ));
                    lines.push(MarkdownLine {
                        spans,
                        is_code_block: false,
                        is_blockquote: false,
                        is_table_row: false,
                    });
                } else if line.is_empty() {
                    lines.push(MarkdownLine {
                        spans: vec![MarkdownSpan::Text(Span::raw(""))],
                        is_code_block: false,
                        is_blockquote: false,
                        is_table_row: false,
                    });
                } else {
                    let spans = parse_inline_style_with_links(
                        line,
                        palette,
                        Style::default().fg(palette.text),
                    );
                    lines.push(MarkdownLine {
                        spans,
                        is_code_block: false,
                        is_blockquote: false,
                        is_table_row: false,
                    });
                }
            }
        }

        line_idx += 1;
    }

    if in_code_block && !code_block_lines.is_empty() {
        let highlighted = highlight_code_block(&code_block_lines, &code_block_lang, palette);
        lines.extend(highlighted);
    }

    lines
}

// Allowed because this is a public API kept for backward compatibility and verified in unit tests.
#[allow(dead_code)]
pub fn parse_markdown(text: &str, palette: &Palette) -> Vec<Line<'static>> {
    parse_markdown_with_links(text, palette)
        .into_iter()
        .map(|line| {
            let mut spans = Vec::new();
            for span in line.spans {
                match span {
                    MarkdownSpan::Text(s) => spans.push(s),
                    MarkdownSpan::Link { label_spans, .. } => spans.extend(label_spans),
                }
            }
            Line::from(spans)
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct WrappedMarkdown {
    pub lines: Vec<Line<'static>>,
    pub link_ranges: Vec<(usize, std::ops::Range<usize>, String)>,
}

#[derive(Debug, Clone)]
struct Token {
    text: String,
    style: Style,
    url: Option<String>,
}

pub fn wrap_markdown(lines: &[MarkdownLine], width: usize) -> WrappedMarkdown {
    use unicode_width::UnicodeWidthStr;

    if width == 0 {
        return WrappedMarkdown {
            lines: Vec::new(),
            link_ranges: Vec::new(),
        };
    }

    let mut wrapped_lines = Vec::new();
    let mut link_ranges = Vec::new();

    for md_line in lines {
        if md_line.is_table_row {
            let line_index = wrapped_lines.len();
            let mut current_x = 0;
            let mut line_spans = Vec::new();

            for span in &md_line.spans {
                match span {
                    MarkdownSpan::Text(s) => {
                        let w = UnicodeWidthStr::width(s.content.as_ref());
                        current_x += w;
                        line_spans.push(s.clone());
                    }
                    MarkdownSpan::Link { label_spans, url } => {
                        let start_x = current_x;
                        for s in label_spans {
                            let w = UnicodeWidthStr::width(s.content.as_ref());
                            current_x += w;
                            line_spans.push(s.clone());
                        }
                        let end_x = current_x;
                        link_ranges.push((line_index, start_x..end_x, url.clone()));
                    }
                }
            }
            wrapped_lines.push(Line::from(line_spans));
            continue;
        }

        // Step 1: Tokenize
        let mut tokens = Vec::new();

        let has_prefix = md_line.is_blockquote || md_line.is_code_block;

        let spans_to_tokenize = if has_prefix && !md_line.spans.is_empty() {
            &md_line.spans[1..]
        } else {
            &md_line.spans[..]
        };

        let prefix_span = if has_prefix && !md_line.spans.is_empty() {
            if let MarkdownSpan::Text(ref s) = md_line.spans[0] {
                Some(s.clone())
            } else {
                None
            }
        } else {
            None
        };

        for span in spans_to_tokenize {
            match span {
                MarkdownSpan::Text(s) => {
                    tokenize_span_content(&s.content, s.style, None, &mut tokens);
                }
                MarkdownSpan::Link { label_spans, url } => {
                    for s in label_spans {
                        tokenize_span_content(&s.content, s.style, Some(url.clone()), &mut tokens);
                    }
                }
            }
        }

        // Step 2: Wrap tokens
        let mut current_line_tokens: Vec<(Token, usize)> = Vec::new();
        let mut current_line_width = 0;
        let mut is_first_subline = true;
        let mut committed_any = false;

        let prefix_width = prefix_span
            .as_ref()
            .map(|p| unicode_width::UnicodeWidthStr::width(p.content.as_ref()))
            .unwrap_or(0);

        let wrap_width = width.saturating_sub(prefix_width);

        let mut i = 0;
        while i < tokens.len() {
            let token = &tokens[i];
            let token_width = token.text.width();

            // Handle space skipping on wrapped lines:
            // If this is a continuation line, and it is empty so far, and the token is a word separator (starts with ' '), skip it.
            if !is_first_subline && current_line_width == 0 && token.text.starts_with(' ') {
                i += 1;
                continue;
            }

            if current_line_width + token_width <= wrap_width || current_line_width == 0 {
                // Fits in current line, or it's the first token of the line (forced fit to prevent infinite loop)
                current_line_tokens.push((tokens[i].clone(), current_line_width));
                current_line_width += token_width;
                i += 1;
            } else {
                // Doesn't fit. Commit current line, start new one.
                commit_line(
                    &current_line_tokens,
                    wrapped_lines.len(),
                    &mut wrapped_lines,
                    &mut link_ranges,
                    md_line.is_code_block,
                    width,
                    prefix_span.as_ref(),
                );
                committed_any = true;
                current_line_tokens.clear();
                current_line_width = 0;
                is_first_subline = false;
            }
        }

        // Commit any remaining tokens for this block
        if !current_line_tokens.is_empty() || !committed_any || md_line.spans.is_empty() {
            commit_line(
                &current_line_tokens,
                wrapped_lines.len(),
                &mut wrapped_lines,
                &mut link_ranges,
                md_line.is_code_block,
                width,
                prefix_span.as_ref(),
            );
        }
    }

    WrappedMarkdown {
        lines: wrapped_lines,
        link_ranges,
    }
}

fn tokenize_span_content(
    content: &str,
    style: Style,
    url: Option<String>,
    tokens: &mut Vec<Token>,
) {
    if content.is_empty() {
        return;
    }

    let mut chars = content.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c == ' ' {
            let mut space_run = String::new();
            while let Some(&' ') = chars.peek() {
                space_run.push(chars.next().unwrap());
            }
            tokens.push(Token {
                text: space_run,
                style,
                url: url.clone(),
            });
        } else {
            let mut word = String::new();
            while let Some(&nc) = chars.peek() {
                if nc == ' ' {
                    break;
                }
                word.push(chars.next().unwrap());
            }
            tokens.push(Token {
                text: word,
                style,
                url: url.clone(),
            });
        }
    }
}

fn commit_line(
    tokens_with_offsets: &[(Token, usize)],
    line_index: usize,
    wrapped_lines: &mut Vec<Line<'static>>,
    link_ranges: &mut Vec<(usize, std::ops::Range<usize>, String)>,
    is_code_block: bool,
    width: usize,
    prefix_span: Option<&Span<'static>>,
) {
    let mut spans = Vec::new();
    let mut active_link: Option<(usize, usize, String)> = None;

    if let Some(prefix) = prefix_span {
        spans.push(prefix.clone());
    }

    let prefix_width = prefix_span
        .as_ref()
        .map(|p| unicode_width::UnicodeWidthStr::width(p.content.as_ref()))
        .unwrap_or(0);

    for (token, offset) in tokens_with_offsets {
        spans.push(Span::styled(token.text.clone(), token.style));

        let token_width = unicode_width::UnicodeWidthStr::width(token.text.as_str());
        let token_end = offset + token_width;

        if let Some(ref url) = token.url {
            if let Some((start_col, ref mut end_col, ref active_url)) = active_link {
                if active_url == url {
                    *end_col = token_end;
                } else {
                    link_ranges.push((
                        line_index,
                        (start_col + prefix_width)..(*end_col + prefix_width),
                        active_url.clone(),
                    ));
                    active_link = Some((*offset, token_end, url.clone()));
                }
            } else {
                active_link = Some((*offset, token_end, url.clone()));
            }
        } else {
            if let Some((start_col, end_col, url)) = active_link.take() {
                link_ranges.push((
                    line_index,
                    (start_col + prefix_width)..(end_col + prefix_width),
                    url,
                ));
            }
        }
    }

    if let Some((start_col, end_col, url)) = active_link {
        link_ranges.push((
            line_index,
            (start_col + prefix_width)..(end_col + prefix_width),
            url,
        ));
    }

    let mut current_width = 0;
    for span in &spans {
        current_width += unicode_width::UnicodeWidthStr::width(span.content.as_ref());
    }

    if is_code_block && current_width < width {
        let padding_bg = tokens_with_offsets
            .first()
            .and_then(|(t, _)| t.style.bg)
            .or_else(|| prefix_span.and_then(|p| p.style.bg));
        let mut padding_style = Style::default();
        if let Some(bg) = padding_bg {
            padding_style = padding_style.bg(bg);
        }
        let padding_len = width - current_width;
        spans.push(Span::styled(" ".repeat(padding_len), padding_style));
    }

    wrapped_lines.push(Line::from(spans));
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    fn test_palette() -> Palette {
        Palette {
            accent: Color::Cyan,
            panel_bg: Color::Black,
            surface0: Color::DarkGray,
            surface1: Color::Gray,
            surface_dim: Color::Black,
            overlay0: Color::DarkGray,
            overlay1: Color::Gray,
            text: Color::White,
            subtext0: Color::Gray,
            mauve: Color::Magenta,
            green: Color::Green,
            yellow: Color::Yellow,
            red: Color::Red,
            blue: Color::Blue,
            teal: Color::Cyan,
            peach: Color::Yellow,
        }
    }

    #[test]
    fn test_parse_inline_styles() {
        let palette = test_palette();

        // 1. Plain text
        let spans = parse_inline_style("hello world", &palette, Style::default().fg(Color::White));
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "hello world");
        assert_eq!(spans[0].style.fg, Some(Color::White));

        // 2. Bold text
        let spans = parse_inline_style(
            "hello **world**",
            &palette,
            Style::default().fg(Color::White),
        );
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content, "hello ");
        assert_eq!(spans[1].content, "world");
        assert!(spans[1].style.add_modifier.contains(Modifier::BOLD));

        // 3. Nested bold & italic
        let spans = parse_inline_style(
            "hello **bold *italic* bold**",
            &palette,
            Style::default().fg(Color::White),
        );
        assert_eq!(spans.len(), 4);
        assert_eq!(spans[0].content, "hello ");
        assert_eq!(spans[1].content, "bold ");
        assert_eq!(spans[2].content, "italic");
        assert!(spans[2].style.add_modifier.contains(Modifier::BOLD));
        assert!(spans[2].style.add_modifier.contains(Modifier::ITALIC));
        assert_eq!(spans[3].content, " bold");

        // 4. Code block
        let spans = parse_inline_style("hello `code`", &palette, Style::default().fg(Color::White));
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content, "hello ");
        assert_eq!(spans[1].content, "code");
        assert_eq!(spans[1].style.fg, Some(Color::Cyan));
        assert_eq!(spans[1].style.bg, Some(Color::DarkGray));
        assert!(spans[1].style.add_modifier.contains(Modifier::BOLD));

        // 5. Links
        let spans = parse_inline_style("[label](url)", &palette, Style::default().fg(Color::White));
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "label");
        assert_eq!(spans[0].style.fg, Some(Color::Blue));
        assert!(spans[0].style.add_modifier.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn test_parse_markdown_blocks() {
        let palette = test_palette();

        // Headers
        let lines = parse_markdown("# Header 1\n## Header 2", &palette);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].spans[0].content, "█ ");
        assert_eq!(lines[0].spans[1].content, "Header 1");
        assert_eq!(lines[1].spans[0].content, "Header 2");

        // Lists
        let lines = parse_markdown("- list item\n  - nested", &palette);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].spans[0].content, "");
        assert_eq!(lines[0].spans[1].content, "• ");
        assert_eq!(lines[0].spans[2].content, "list item");

        assert_eq!(lines[1].spans[0].content, "  ");
        assert_eq!(lines[1].spans[1].content, "• ");
        assert_eq!(lines[1].spans[2].content, "nested");

        // Code block
        let lines = parse_markdown("```rust\nfn main() {}\n```", &palette);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans[0].content, "▏");
        let code_text: String = lines[0].spans[1..]
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(code_text, "fn main() {}");

        // Horizontal rules
        let lines = parse_markdown("---", &palette);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans[0].content, "─".repeat(40));
    }

    #[test]
    fn test_syntax_highlighted_code_block() {
        let palette = test_palette();

        // Verify Rust code block syntax highlighting has multiple spans with different styling
        let md_lines =
            parse_markdown_with_links("```rust\nfn main() {\n    // comment\n}\n```", &palette);
        assert_eq!(md_lines.len(), 3);
        assert!(md_lines[0].is_code_block);

        // First line: "fn main() {"
        let line0 = &md_lines[0];
        assert_eq!(
            line0.spans[0],
            MarkdownSpan::Text(Span::styled(
                "▏",
                Style::default().fg(palette.accent).bg(palette.surface1)
            ))
        );

        // Assert there are multiple spans because of syntax tokenization
        assert!(line0.spans.len() > 2);

        // Find the "fn" keyword span
        let fn_span = line0.spans.iter().find(|s| match s {
            MarkdownSpan::Text(span) => span.content == "fn",
            _ => false,
        });
        assert!(fn_span.is_some(), "Should find 'fn' keyword span");

        // Second line: "    // comment"
        let line1 = &md_lines[1];
        let code_text1: String = line1.spans[1..]
            .iter()
            .map(|s| match s {
                MarkdownSpan::Text(span) => span.content.as_ref(),
                _ => "",
            })
            .collect();
        assert!(code_text1.contains("// comment"));
    }

    #[test]
    fn test_parse_markdown_task_lists() {
        let palette = test_palette();

        // Task lists: unchecked and checked
        let lines = parse_markdown(
            "- [ ] Todo item\n* [x] Done item\n+ [X] Another done\n  - [ ] Nested todo",
            &palette,
        );
        assert_eq!(lines.len(), 4);

        // First item: "- [ ] Todo item"
        assert_eq!(lines[0].spans[0].content, "");
        assert_eq!(lines[0].spans[1].content, "[ ] ");
        assert_eq!(lines[0].spans[1].style.fg, Some(palette.overlay1));
        assert_eq!(lines[0].spans[2].content, "Todo item");

        // Second item: "* [x] Done item"
        assert_eq!(lines[1].spans[0].content, "");
        assert_eq!(lines[1].spans[1].content, "[✓] ");
        assert_eq!(lines[1].spans[1].style.fg, Some(palette.green));
        assert_eq!(lines[1].spans[2].content, "Done item");
        assert!(lines[1].spans[2]
            .style
            .add_modifier
            .contains(Modifier::CROSSED_OUT));
        assert_eq!(lines[1].spans[2].style.fg, Some(palette.subtext0));

        // Third item: "+ [X] Another done"
        assert_eq!(lines[2].spans[0].content, "");
        assert_eq!(lines[2].spans[1].content, "[✓] ");
        assert_eq!(lines[2].spans[1].style.fg, Some(palette.green));
        assert_eq!(lines[2].spans[2].content, "Another done");
        assert!(lines[2].spans[2]
            .style
            .add_modifier
            .contains(Modifier::CROSSED_OUT));

        // Fourth item: "  - [ ] Nested todo"
        assert_eq!(lines[3].spans[0].content, "  ");
        assert_eq!(lines[3].spans[1].content, "[ ] ");
        assert_eq!(lines[3].spans[2].content, "Nested todo");
    }

    #[test]
    fn test_parse_markdown_with_links() {
        let palette = test_palette();
        let md_lines =
            parse_markdown_with_links("hello [link label](http://example.com) world", &palette);
        assert_eq!(md_lines.len(), 1);

        let spans = &md_lines[0].spans;
        assert_eq!(spans.len(), 3);

        assert_eq!(
            spans[0],
            MarkdownSpan::Text(Span::styled("hello ", Style::default().fg(palette.text)))
        );

        match &spans[1] {
            MarkdownSpan::Link { label_spans, url } => {
                assert_eq!(url, "http://example.com");
                assert_eq!(label_spans.len(), 1);
                assert_eq!(label_spans[0].content, "link label");
                assert_eq!(label_spans[0].style.fg, Some(palette.blue));
                assert!(label_spans[0]
                    .style
                    .add_modifier
                    .contains(Modifier::UNDERLINED));
            }
            _ => panic!("Expected MarkdownSpan::Link"),
        }

        assert_eq!(
            spans[2],
            MarkdownSpan::Text(Span::styled(" world", Style::default().fg(palette.text)))
        );
    }

    #[test]
    fn test_wrap_markdown_with_links() {
        let palette = test_palette();
        let md_lines = parse_markdown_with_links("hello [link](http://foo) world", &palette);

        let wrapped = wrap_markdown(&md_lines, 12);

        assert_eq!(wrapped.lines.len(), 2);
        assert_eq!(wrapped.lines[0].spans[0].content, "hello");
        assert_eq!(wrapped.lines[0].spans[1].content, " ");
        assert_eq!(wrapped.lines[0].spans[2].content, "link");

        assert_eq!(wrapped.link_ranges.len(), 1);
        assert_eq!(wrapped.link_ranges[0], (0, 6..10, "http://foo".to_string()));

        assert_eq!(wrapped.lines[1].spans[0].content, "world");
    }

    #[test]
    fn test_wrap_markdown_code_block_padding() {
        let palette = test_palette();
        let md_lines = parse_markdown_with_links("```rust\nfn main() {}\n```", &palette);

        let wrapped = wrap_markdown(&md_lines, 20);

        assert_eq!(wrapped.lines.len(), 1);
        let line = &wrapped.lines[0];
        // The line is split into tokens (words and spaces) by the wrapper
        let line_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(line_text, "▏fn main() {}       ");
        assert_eq!(
            unicode_width::UnicodeWidthStr::width(line_text.as_str()),
            20
        ); // 1 (▏) + 12 (fn main() {}) + 7 (padding)

        // Verify background colors are correctly set to surface1
        for (i, span) in line.spans.iter().enumerate() {
            assert_eq!(
                span.style.bg,
                Some(palette.surface1),
                "span {} ({:?}) has wrong bg",
                i,
                span.content
            );
        }
    }

    #[test]
    fn test_wrap_markdown_blockquote_wrapping() {
        let palette = test_palette();
        let md_lines = parse_markdown_with_links(
            "> This is a very long blockquote line that wraps.",
            &palette,
        );

        // Wrap to width 20. "│ " takes 2 characters, so wrap_width is 18.
        let wrapped = wrap_markdown(&md_lines, 20);

        assert_eq!(wrapped.lines.len(), 3);

        for (i, line) in wrapped.lines.iter().enumerate() {
            let line_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            assert!(
                line_text.starts_with("│ "),
                "line {} does not start with quote prefix: {:?}",
                i,
                line_text
            );
            assert!(
                unicode_width::UnicodeWidthStr::width(line_text.as_str()) <= 20,
                "line {} too wide: {} columns",
                i,
                unicode_width::UnicodeWidthStr::width(line_text.as_str())
            );
        }

        let line0_text: String = wrapped.lines[0]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        let line1_text: String = wrapped.lines[1]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        let line2_text: String = wrapped.lines[2]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();

        assert_eq!(line0_text, "│ This is a very ");
        assert_eq!(line1_text, "│ long blockquote ");
        assert_eq!(line2_text, "│ line that wraps.");
    }

    #[test]
    fn test_wrap_markdown_code_block_wrapping() {
        let palette = test_palette();
        let md_lines = parse_markdown_with_links("```rust\nlet variable = 123456;\n```", &palette);

        // Wrap to width 15. "▏" takes 1 character, so wrap_width is 14.
        let wrapped = wrap_markdown(&md_lines, 15);

        assert_eq!(wrapped.lines.len(), 2);

        for (i, line) in wrapped.lines.iter().enumerate() {
            let line_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            assert!(
                line_text.starts_with("▏"),
                "line {} does not start with code border: {:?}",
                i,
                line_text
            );
            assert_eq!(
                unicode_width::UnicodeWidthStr::width(line_text.as_str()),
                15,
                "line {} does not extend across to full width: {:?}",
                i,
                line_text
            );
            // Verify background colors are correctly set to surface1
            for (j, span) in line.spans.iter().enumerate() {
                assert_eq!(
                    span.style.bg,
                    Some(palette.surface1),
                    "line {}, span {} ({:?}) has wrong bg",
                    i,
                    j,
                    span.content
                );
            }
        }

        let line0_text: String = wrapped.lines[0]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        let line1_text: String = wrapped.lines[1]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();

        assert_eq!(line0_text, "▏let variable =");
        assert_eq!(line1_text, "▏123456;       ");
    }

    #[test]
    fn test_markdown_tables() {
        let palette = test_palette();

        // 1. Delimiter alignment and header parsing
        let md = "\
| Left | Center | Right | Default |
| :--- | :---:  | ----: | ------- |
| val1 | val2   | val3  | val4    |";

        let lines = parse_markdown_with_links(md, &palette);
        assert_eq!(lines.len(), 5);
        assert!(lines[0].is_table_row);
        assert!(lines[1].is_table_row);
        assert!(lines[2].is_table_row);
        assert!(lines[3].is_table_row);
        assert!(lines[4].is_table_row);

        let line0_text: String = lines[0]
            .spans
            .iter()
            .map(|s| match s {
                MarkdownSpan::Text(span) => span.content.as_ref(),
                _ => "",
            })
            .collect();
        assert_eq!(line0_text, "┌──────┬────────┬───────┬─────────┐");

        let line2_text: String = lines[2]
            .spans
            .iter()
            .map(|s| match s {
                MarkdownSpan::Text(span) => span.content.as_ref(),
                _ => "",
            })
            .collect();
        assert_eq!(line2_text, "├──────┼────────┼───────┼─────────┤");

        let line4_text: String = lines[4]
            .spans
            .iter()
            .map(|s| match s {
                MarkdownSpan::Text(span) => span.content.as_ref(),
                _ => "",
            })
            .collect();
        assert_eq!(line4_text, "└──────┴────────┴───────┴─────────┘");

        // 2. Alignment of headers and values
        let header_row_text: String = lines[1]
            .spans
            .iter()
            .map(|s| match s {
                MarkdownSpan::Text(span) => span.content.as_ref(),
                _ => "",
            })
            .collect();
        assert_eq!(header_row_text, "│ Left │ Center │ Right │ Default │");

        let body_row_text: String = lines[3]
            .spans
            .iter()
            .map(|s| match s {
                MarkdownSpan::Text(span) => span.content.as_ref(),
                _ => "",
            })
            .collect();
        assert_eq!(body_row_text, "│ val1 │  val2  │  val3 │ val4    │");

        // 3. Spans with inline formatting inside table cells
        let md_formatted = "\
| **Bold** | `Code` | [Link](url) |
| --- | --- | --- |
| val1 | val2 | val3 |";
        let lines_fmt = parse_markdown_with_links(md_formatted, &palette);
        assert_eq!(lines_fmt.len(), 5);

        let bold_span = &lines_fmt[1].spans[2]; // border '│', padding ' ', then Bold spans
        match bold_span {
            MarkdownSpan::Text(span) => {
                assert_eq!(span.content.as_ref(), "Bold");
                assert!(span
                    .style
                    .add_modifier
                    .contains(ratatui::style::Modifier::BOLD));
            }
            _ => panic!("Expected text span"),
        }

        let link_span = &lines_fmt[1].spans[10];
        match link_span {
            MarkdownSpan::Link { label_spans, url } => {
                assert_eq!(url, "url");
                assert_eq!(label_spans[0].content.as_ref(), "Link");
            }
            _ => panic!("Expected link span"),
        }

        // 4. Wrapping behavior: tables should be unwrapped!
        let wrapped = wrap_markdown(&lines, 10);
        assert_eq!(wrapped.lines.len(), 5);
        let wrapped_line3: String = wrapped.lines[3]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(wrapped_line3, "│ val1 │  val2  │  val3 │ val4    │");

        // 5. Escaped pipes handling
        let md_escaped = "\
| Escaped \\| Pipe | Col2 |
| --- | --- |
| val1 | val2 |";
        let lines_escaped = parse_markdown_with_links(md_escaped, &palette);
        assert_eq!(lines_escaped.len(), 5);
        let esc_header: String = lines_escaped[1]
            .spans
            .iter()
            .map(|s| match s {
                MarkdownSpan::Text(span) => span.content.as_ref(),
                _ => "",
            })
            .collect();
        assert_eq!(esc_header, "│ Escaped | Pipe │ Col2 │");
    }
}
