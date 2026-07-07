use crossterm::event::{Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};

use crate::app::state::{Palette, THEME_NAMES};
use crate::kitty_graphics::{
    clear_standalone_placements, detect_support, paint_static_placements_standalone, HostCellSize,
    HostGraphicsCache,
};
use crate::platform::{open_url, write_clipboard};
use crate::ui::{
    MarkdownDocument, MarkdownPreview, MarkdownPreviewRequest, MarkdownPreviewScrollbars,
};

pub fn run_md_command(args: &[String]) -> std::io::Result<i32> {
    let mut theme_name = "catppuccin".to_string();
    let mut file_path: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--theme" | "-t" => {
                if i + 1 < args.len() {
                    theme_name = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("error: --theme requires a value");
                    return Ok(1);
                }
            }
            "--list-themes" | "-l" => {
                println!("Available themes:");
                for theme in THEME_NAMES {
                    println!("  {}", theme);
                }
                return Ok(0);
            }
            "--help" | "-h" => {
                print_md_help();
                return Ok(0);
            }
            arg if arg.starts_with('-') => {
                eprintln!("error: unknown option: {}", arg);
                eprintln!("run 'herdr md --help' for usage");
                return Ok(1);
            }
            arg => {
                if file_path.is_some() {
                    eprintln!("error: multiple input files specified");
                    return Ok(1);
                }
                file_path = Some(arg.to_string());
                i += 1;
            }
        }
    }

    let (content, source_name) = match file_path.as_deref() {
        None => {
            use std::io::IsTerminal;
            if std::io::stdin().is_terminal() {
                eprintln!("error: no input file specified");
                eprintln!("run 'herdr md --help' for usage");
                return Ok(1);
            }
            let mut buf = String::new();
            use std::io::Read;
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| std::io::Error::other(format!("failed to read stdin: {e}")))?;

            #[cfg(unix)]
            {
                use std::os::fd::AsRawFd;
                if let Ok(tty) = std::fs::File::open("/dev/tty") {
                    unsafe {
                        libc::dup2(tty.as_raw_fd(), 0);
                    }
                }
            }

            (buf, "stdin".to_string())
        }
        Some("-") => {
            let mut buf = String::new();
            use std::io::Read;
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| std::io::Error::other(format!("failed to read stdin: {e}")))?;

            #[cfg(unix)]
            {
                use std::os::fd::AsRawFd;
                if let Ok(tty) = std::fs::File::open("/dev/tty") {
                    unsafe {
                        libc::dup2(tty.as_raw_fd(), 0);
                    }
                }
            }

            (buf, "stdin".to_string())
        }
        Some(path) => {
            let content = std::fs::read_to_string(path)
                .map_err(|e| std::io::Error::other(format!("failed to read file '{path}': {e}")))?;
            (content, path.to_string())
        }
    };

    let palette = Palette::from_name(&theme_name).unwrap_or_else(|| {
        eprintln!(
            "warning: theme '{}' not found, defaulting to catppuccin",
            theme_name
        );
        Palette::catppuccin()
    });

    let mut terminal = ratatui::init();
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;
    let mut scroll_y: u16 = 0;
    let mut scroll_x: u16 = 0;
    let mut status_message: Option<(String, std::time::Instant)> = None;

    let mut graphics_cache = HostGraphicsCache::default();
    let kitty_graphics_enabled = detect_support();

    let mut doc = MarkdownDocument::new();
    doc.append_markdown(&content, &palette);

    let mut result = Ok(());

    loop {
        let size = match terminal.size() {
            Ok(size) => size,
            Err(e) => {
                result = Err(e);
                break;
            }
        };
        let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);
        if area.height < 2 || area.width < 3 {
            std::thread::sleep(std::time::Duration::from_millis(50));
            continue;
        }

        let cell_size = if kitty_graphics_enabled {
            HostCellSize::from_terminal(area)
        } else {
            HostCellSize::default()
        };

        // Split layout: content and status bar
        let chunks = ratatui::layout::Layout::vertical([
            ratatui::layout::Constraint::Min(0),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(area);

        let content_area = chunks[0];
        let status_area = chunks[1];

        let preview_area = ratatui::layout::Rect::new(
            content_area.x + 1,
            content_area.y,
            content_area.width.saturating_sub(1),
            content_area.height,
        );

        let preview = MarkdownPreview::build(MarkdownPreviewRequest {
            document: &doc,
            area: preview_area,
            scroll_y,
            scroll_x,
            cell_size,
            text_color: palette.text,
            scrollbars: MarkdownPreviewScrollbars::BOTH,
        });

        // Keep scroll within bounds
        scroll_y = preview.scroll_y;
        scroll_x = preview.scroll_x;

        // Render
        let draw_res = terminal.draw(|f| {
            // Render background
            f.render_widget(
                ratatui::widgets::Block::default()
                    .style(ratatui::style::Style::default().bg(palette.panel_bg)),
                area,
            );

            // Render Markdown text
            let lines = preview.lines();
            let paragraph = ratatui::widgets::Paragraph::new(lines.to_vec())
                .scroll((scroll_y, 0))
                .style(ratatui::style::Style::default().fg(palette.text));
            f.render_widget(paragraph, preview.text_area);

            // Vertical scrollbar rendering
            if let Some(scrollbar) = preview.vertical_scrollbar {
                let track_height = scrollbar.track.height;
                let total_lines = lines.len();
                let thumb_height = (((track_height as f32) * (track_height as f32))
                    / (total_lines as f32))
                    .clamp(1.0, track_height as f32) as u16;
                let scrollable_height = track_height.saturating_sub(thumb_height);
                let thumb_y = if preview.max_scroll_y > 0 {
                    ((scroll_y as f32 / preview.max_scroll_y as f32) * (scrollable_height as f32))
                        as u16
                } else {
                    0
                };
                for y in 0..track_height {
                    let is_thumb = y >= thumb_y && y < thumb_y + thumb_height;
                    let char_to_draw = if is_thumb { "┃" } else { "│" };
                    let style = if is_thumb {
                        ratatui::style::Style::default().fg(palette.accent)
                    } else {
                        ratatui::style::Style::default().fg(palette.surface1)
                    };
                    let cell_area =
                        ratatui::layout::Rect::new(scrollbar.track.x, scrollbar.track.y + y, 1, 1);
                    f.render_widget(
                        ratatui::widgets::Paragraph::new(char_to_draw).style(style),
                        cell_area,
                    );
                }
            }

            // Horizontal scrollbar rendering
            if let Some(scrollbar) = preview.horizontal_scrollbar {
                let track_width = scrollbar.track.width;
                let max_width = scrollbar.content_width;
                let thumb_width = (((track_width as f32) * (track_width as f32))
                    / (max_width as f32))
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
                    let char_to_draw = if is_thumb { "━" } else { "─" };
                    let style = if is_thumb {
                        ratatui::style::Style::default().fg(palette.accent)
                    } else {
                        ratatui::style::Style::default().fg(palette.surface1)
                    };
                    let cell_area =
                        ratatui::layout::Rect::new(scrollbar.track.x + x, scrollbar.track.y, 1, 1);
                    f.render_widget(
                        ratatui::widgets::Paragraph::new(char_to_draw).style(style),
                        cell_area,
                    );
                }
            }

            // Status Bar
            let percent = if preview.max_scroll_y > 0 {
                (scroll_y as f32 / preview.max_scroll_y as f32 * 100.0) as u32
            } else {
                100
            };

            let now = std::time::Instant::now();
            let left_status = if let Some((ref msg, timestamp)) = status_message {
                if now.duration_since(timestamp) < std::time::Duration::from_secs(2) {
                    format!(" {} ", msg)
                } else {
                    format!(" herdr md: {} ", source_name)
                }
            } else {
                format!(" herdr md: {} ", source_name)
            };

            let right_status = format!(
                " theme: {} | Line: {}/{} ({}%) | q: Quit ",
                theme_name,
                scroll_y + 1,
                lines.len(),
                percent
            );

            let status_style = ratatui::style::Style::default()
                .fg(palette.panel_bg)
                .bg(palette.accent)
                .add_modifier(ratatui::style::Modifier::BOLD);

            let status_para_left = ratatui::widgets::Paragraph::new(left_status)
                .style(status_style)
                .alignment(ratatui::layout::Alignment::Left);
            let status_para_right = ratatui::widgets::Paragraph::new(right_status)
                .style(status_style)
                .alignment(ratatui::layout::Alignment::Right);

            f.render_widget(status_para_left, status_area);
            f.render_widget(status_para_right, status_area);
        });

        if let Err(e) = draw_res {
            result = Err(e);
            break;
        }

        if kitty_graphics_enabled && cell_size.is_known() {
            let mut static_placements = Vec::new();
            preview.push_image_placements(&mut static_placements);
            let _ = paint_static_placements_standalone(
                &static_placements,
                cell_size,
                &mut graphics_cache,
            );
        }

        // Read event
        match crossterm::event::poll(std::time::Duration::from_millis(100)) {
            Ok(true) => {
                match crossterm::event::read() {
                    Ok(Event::Key(key)) => {
                        // Check for exit
                        if key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            break;
                        }
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => break,
                            // Vertical Scroll
                            KeyCode::Down | KeyCode::Char('j') => {
                                scroll_y = scroll_y.saturating_add(1);
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                scroll_y = scroll_y.saturating_sub(1);
                            }
                            KeyCode::PageDown | KeyCode::Char('d')
                                if key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                scroll_y = scroll_y.saturating_add(area.height.saturating_sub(2));
                            }
                            KeyCode::PageDown => {
                                scroll_y = scroll_y.saturating_add(area.height.saturating_sub(2));
                            }
                            KeyCode::PageUp | KeyCode::Char('u')
                                if key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                scroll_y = scroll_y.saturating_sub(area.height.saturating_sub(2));
                            }
                            KeyCode::PageUp => {
                                scroll_y = scroll_y.saturating_sub(area.height.saturating_sub(2));
                            }
                            KeyCode::Home => {
                                scroll_y = 0;
                                scroll_x = 0;
                            }
                            KeyCode::End => {
                                scroll_y = u16::MAX;
                            }
                            // Horizontal Scroll
                            KeyCode::Right | KeyCode::Char('l') => {
                                scroll_x = scroll_x.saturating_add(4);
                            }
                            KeyCode::Left | KeyCode::Char('h') => {
                                scroll_x = scroll_x.saturating_sub(4);
                            }
                            _ => {}
                        }
                    }
                    Ok(Event::Mouse(mouse)) => match mouse.kind {
                        MouseEventKind::ScrollDown => {
                            if mouse.modifiers.contains(KeyModifiers::SHIFT) {
                                scroll_x = scroll_x.saturating_add(4);
                            } else {
                                scroll_y = scroll_y.saturating_add(3);
                            }
                        }
                        MouseEventKind::ScrollUp => {
                            if mouse.modifiers.contains(KeyModifiers::SHIFT) {
                                scroll_x = scroll_x.saturating_sub(4);
                            } else {
                                scroll_y = scroll_y.saturating_sub(3);
                            }
                        }
                        MouseEventKind::Down(MouseButton::Left) => {
                            let active_links = preview.active_hyperlinks();
                            if let Some((_, _, url)) = active_links
                                .iter()
                                .find(|((x, y), _, _)| *x == mouse.column && *y == mouse.row)
                            {
                                let is_web =
                                    url.starts_with("http://") || url.starts_with("https://");
                                let mut success = false;
                                if is_web && open_url(url).is_ok() {
                                    success = true;
                                    status_message = Some((
                                        format!("Opened link: {}", url),
                                        std::time::Instant::now(),
                                    ));
                                }
                                if !success {
                                    if write_clipboard(url.as_bytes()) {
                                        status_message = Some((
                                            format!("Copied link to clipboard: {}", url),
                                            std::time::Instant::now(),
                                        ));
                                    } else {
                                        status_message = Some((
                                            format!("Failed to open/copy link: {}", url),
                                            std::time::Instant::now(),
                                        ));
                                    }
                                }
                            }
                        }
                        _ => {}
                    },
                    Ok(Event::Resize(_, _)) => {}
                    _ => {}
                }
            }
            Ok(false) => {}
            Err(e) => {
                result = Err(e);
                break;
            }
        }
    }

    if kitty_graphics_enabled {
        let _ = clear_standalone_placements(&mut graphics_cache);
    }
    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture)?;
    ratatui::restore();

    result.map(|_| 0)
}

fn print_md_help() {
    println!("herdr md — Render markdown and LaTeX math formulas in TUI");
    println!();
    println!("Usage: herdr md [options] [file]");
    println!();
    println!("Arguments:");
    println!(
        "  [file]            The markdown file to render. Use '-' or omit to read from stdin."
    );
    println!();
    println!("Options:");
    println!("  -t, --theme <name>  Select color theme (default: catppuccin)");
    println!("  -l, --list-themes   List all available themes and exit");
    println!("  -h, --help          Show this help text");
}
