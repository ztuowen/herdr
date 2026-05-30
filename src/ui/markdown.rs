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

pub fn parse_markdown_with_links(text: &str, palette: &Palette) -> Vec<MarkdownLine> {
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
                MarkdownSpan::Text(Span::styled(
                    "▏",
                    Style::default().fg(palette.accent).bg(palette.surface1),
                )),
                MarkdownSpan::Text(Span::styled(
                    line.to_string(),
                    Style::default().fg(palette.text).bg(palette.surface1),
                )),
            ];
            lines.push(MarkdownLine { spans });
            continue;
        }

        // Check for horizontal rule
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            lines.push(MarkdownLine {
                spans: vec![MarkdownSpan::Text(Span::styled(
                    "─".repeat(40),
                    Style::default().fg(palette.surface0),
                ))],
            });
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
            lines.push(MarkdownLine { spans });
        } else if let Some(content) = line.strip_prefix("## ") {
            let spans = parse_inline_style_with_links(
                content,
                palette,
                Style::default()
                    .fg(palette.teal)
                    .add_modifier(Modifier::BOLD),
            );
            lines.push(MarkdownLine { spans });
        } else if let Some(content) = line.strip_prefix("### ") {
            let spans = parse_inline_style_with_links(
                content,
                palette,
                Style::default()
                    .fg(palette.peach)
                    .add_modifier(Modifier::BOLD),
            );
            lines.push(MarkdownLine { spans });
        } else if let Some(content) = line.strip_prefix("#### ") {
            let spans = parse_inline_style_with_links(
                content,
                palette,
                Style::default()
                    .fg(palette.mauve)
                    .add_modifier(Modifier::BOLD),
            );
            lines.push(MarkdownLine { spans });
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
            lines.push(MarkdownLine { spans });
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
                    lines.push(MarkdownLine { spans });
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
                    lines.push(MarkdownLine { spans });
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
                    lines.push(MarkdownLine { spans });
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
                    lines.push(MarkdownLine { spans });
                } else if line.is_empty() {
                    lines.push(MarkdownLine {
                        spans: vec![MarkdownSpan::Text(Span::raw(""))],
                    });
                } else {
                    let spans = parse_inline_style_with_links(
                        line,
                        palette,
                        Style::default().fg(palette.text),
                    );
                    lines.push(MarkdownLine { spans });
                }
            }
        }
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
        // Step 1: Tokenize
        let mut tokens = Vec::new();
        for span in &md_line.spans {
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

            if current_line_width + token_width <= width || current_line_width == 0 {
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
                );
                current_line_tokens.clear();
                current_line_width = 0;
                is_first_subline = false;
            }
        }

        // Commit any remaining tokens for this block
        if !current_line_tokens.is_empty() || md_line.spans.is_empty() {
            commit_line(
                &current_line_tokens,
                wrapped_lines.len(),
                &mut wrapped_lines,
                &mut link_ranges,
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
) {
    let mut spans = Vec::new();
    let mut active_link: Option<(usize, usize, String)> = None;

    for (token, offset) in tokens_with_offsets {
        spans.push(Span::styled(token.text.clone(), token.style));

        let token_width = unicode_width::UnicodeWidthStr::width(token.text.as_str());
        let token_end = offset + token_width;

        if let Some(ref url) = token.url {
            if let Some((start_col, ref mut end_col, ref active_url)) = active_link {
                if active_url == url {
                    *end_col = token_end;
                } else {
                    link_ranges.push((line_index, start_col..*end_col, active_url.clone()));
                    active_link = Some((*offset, token_end, url.clone()));
                }
            } else {
                active_link = Some((*offset, token_end, url.clone()));
            }
        } else {
            if let Some((start_col, end_col, url)) = active_link.take() {
                link_ranges.push((line_index, start_col..end_col, url));
            }
        }
    }

    if let Some((start_col, end_col, url)) = active_link {
        link_ranges.push((line_index, start_col..end_col, url));
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
        assert_eq!(lines[0].spans[1].content, "fn main() {}");

        // Horizontal rules
        let lines = parse_markdown("---", &palette);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans[0].content, "─".repeat(40));
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
}
