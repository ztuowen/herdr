use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::app::state::Palette;

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

pub(crate) fn parse_inline_style(
    text: &str,
    palette: &Palette,
    current_style: Style,
) -> Vec<Span<'static>> {
    let chars: Vec<char> = text.chars().collect();
    parse_inline_chars(&chars, palette, current_style)
}

pub fn parse_markdown(text: &str, palette: &Palette) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut in_code_block = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            let spans = vec![
                Span::styled(
                    "▏",
                    Style::default().fg(palette.accent).bg(palette.surface1),
                ),
                Span::styled(
                    line.to_string(),
                    Style::default().fg(palette.text).bg(palette.surface1),
                ),
            ];
            lines.push(Line::from(spans));
            continue;
        }

        // Check for horizontal rule
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            lines.push(Line::from(vec![Span::styled(
                "─".repeat(40),
                Style::default().fg(palette.surface0),
            )]));
            continue;
        }

        // Check for headers
        if let Some(content) = line.strip_prefix("# ") {
            let mut spans = vec![Span::styled(
                "█ ",
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            )];
            spans.extend(parse_inline_style(
                content,
                palette,
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            ));
            lines.push(Line::from(spans));
        } else if let Some(content) = line.strip_prefix("## ") {
            let spans = parse_inline_style(
                content,
                palette,
                Style::default()
                    .fg(palette.teal)
                    .add_modifier(Modifier::BOLD),
            );
            lines.push(Line::from(spans));
        } else if let Some(content) = line.strip_prefix("### ") {
            let spans = parse_inline_style(
                content,
                palette,
                Style::default()
                    .fg(palette.peach)
                    .add_modifier(Modifier::BOLD),
            );
            lines.push(Line::from(spans));
        } else if let Some(content) = line.strip_prefix("#### ") {
            let spans = parse_inline_style(
                content,
                palette,
                Style::default()
                    .fg(palette.mauve)
                    .add_modifier(Modifier::BOLD),
            );
            lines.push(Line::from(spans));
        } else if line == ">" || line.starts_with("> ") {
            // Blockquote
            let content = line.strip_prefix("> ").unwrap_or("");
            let mut spans = vec![Span::styled("│ ", Style::default().fg(palette.accent))];
            spans.extend(parse_inline_style(
                content,
                palette,
                Style::default()
                    .fg(palette.overlay1)
                    .add_modifier(Modifier::ITALIC),
            ));
            lines.push(Line::from(spans));
        } else {
            // Check for lists
            let indent_len = line.chars().take_while(|&c| c == ' ').count();
            let suffix = &line[indent_len..];

            if suffix.starts_with("- ") || suffix.starts_with("* ") || suffix.starts_with("+ ") {
                let content = &suffix[2..];
                let indent_str = " ".repeat(indent_len);
                let mut spans = vec![
                    Span::styled(indent_str, Style::default()),
                    Span::styled("• ", Style::default().fg(palette.accent)),
                ];
                spans.extend(parse_inline_style(
                    content,
                    palette,
                    Style::default().fg(palette.text),
                ));
                lines.push(Line::from(spans));
            } else {
                // Check for ordered list
                let digit_chars: String =
                    suffix.chars().take_while(|c| c.is_ascii_digit()).collect();
                if !digit_chars.is_empty() && suffix[digit_chars.len()..].starts_with(". ") {
                    let num_len = digit_chars.len();
                    let content = &suffix[num_len + 2..];
                    let indent_str = " ".repeat(indent_len);
                    let mut spans = vec![
                        Span::styled(indent_str, Style::default()),
                        Span::styled(
                            format!("{}. ", digit_chars),
                            Style::default().fg(palette.accent),
                        ),
                    ];
                    spans.extend(parse_inline_style(
                        content,
                        palette,
                        Style::default().fg(palette.text),
                    ));
                    lines.push(Line::from(spans));
                } else if line.is_empty() {
                    lines.push(Line::from(vec![Span::raw("")]));
                } else {
                    let spans =
                        parse_inline_style(line, palette, Style::default().fg(palette.text));
                    lines.push(Line::from(spans));
                }
            }
        }
    }

    lines
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
        assert_eq!(lines[0].spans[1].content, "fn main() {}");

        // Horizontal rules
        let lines = parse_markdown("---", &palette);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans[0].content, "─".repeat(40));
    }
}
