// Mock/Simplified theme palette module to satisfy upstream markdown.rs imports
mod app {
    pub mod state {
        use ratatui::style::Color;

        #[derive(Clone)]
        pub struct Palette {
            pub accent: Color,
            pub panel_bg: Color,
            pub surface0: Color,
            pub surface1: Color,
            pub surface_dim: Color,
            pub overlay0: Color,
            pub overlay1: Color,
            pub text: Color,
            pub subtext0: Color,
            pub mauve: Color,
            pub green: Color,
            pub yellow: Color,
            pub red: Color,
            pub blue: Color,
            pub teal: Color,
            pub peach: Color,
        }

        impl Palette {
            pub fn catppuccin() -> Self {
                Self {
                    accent: Color::Rgb(137, 180, 250), // blue
                    panel_bg: Color::Rgb(24, 24, 37),
                    surface0: Color::Rgb(49, 50, 68),
                    surface1: Color::Rgb(69, 71, 90),
                    surface_dim: Color::Rgb(30, 30, 46),
                    overlay0: Color::Rgb(108, 112, 134),
                    overlay1: Color::Rgb(127, 132, 156),
                    text: Color::Rgb(205, 214, 244),
                    subtext0: Color::Rgb(166, 173, 200),
                    mauve: Color::Rgb(203, 166, 247),
                    green: Color::Rgb(166, 227, 161),
                    yellow: Color::Rgb(249, 226, 175),
                    red: Color::Rgb(243, 139, 168),
                    blue: Color::Rgb(137, 180, 250),
                    teal: Color::Rgb(148, 226, 213),
                    peach: Color::Rgb(250, 179, 135),
                }
            }

            pub fn terminal() -> Self {
                Self {
                    accent: Color::Blue,
                    panel_bg: Color::Reset,
                    surface0: Color::Reset,
                    surface1: Color::DarkGray,
                    surface_dim: Color::DarkGray,
                    overlay0: Color::Gray,
                    overlay1: Color::White,
                    text: Color::Reset,
                    subtext0: Color::Gray,
                    mauve: Color::Gray,
                    green: Color::Green,
                    yellow: Color::Yellow,
                    red: Color::LightRed,
                    blue: Color::Blue,
                    teal: Color::Cyan,
                    peach: Color::Yellow,
                }
            }

            pub fn tokyo_night() -> Self {
                Self {
                    accent: Color::Rgb(122, 162, 247),
                    panel_bg: Color::Rgb(26, 27, 38),
                    surface0: Color::Rgb(36, 40, 59),
                    surface1: Color::Rgb(65, 72, 104),
                    surface_dim: Color::Rgb(26, 27, 38),
                    overlay0: Color::Rgb(86, 95, 137),
                    overlay1: Color::Rgb(105, 113, 150),
                    text: Color::Rgb(192, 202, 245),
                    subtext0: Color::Rgb(169, 177, 214),
                    mauve: Color::Rgb(187, 154, 247),
                    green: Color::Rgb(158, 206, 106),
                    yellow: Color::Rgb(224, 175, 104),
                    red: Color::Rgb(247, 118, 142),
                    blue: Color::Rgb(122, 162, 247),
                    teal: Color::Rgb(125, 207, 255),
                    peach: Color::Rgb(255, 158, 100),
                }
            }

            pub fn dracula() -> Self {
                Self {
                    accent: Color::Rgb(189, 147, 249),
                    panel_bg: Color::Rgb(40, 42, 54),
                    surface0: Color::Rgb(68, 71, 90),
                    surface1: Color::Rgb(98, 114, 164),
                    surface_dim: Color::Rgb(40, 42, 54),
                    overlay0: Color::Rgb(98, 114, 164),
                    overlay1: Color::Rgb(130, 140, 180),
                    text: Color::Rgb(248, 248, 242),
                    subtext0: Color::Rgb(210, 210, 220),
                    mauve: Color::Rgb(255, 121, 198),
                    green: Color::Rgb(80, 250, 123),
                    yellow: Color::Rgb(241, 250, 140),
                    red: Color::Rgb(255, 85, 85),
                    blue: Color::Rgb(139, 233, 253),
                    teal: Color::Rgb(139, 233, 253),
                    peach: Color::Rgb(255, 184, 108),
                }
            }

            pub fn nord() -> Self {
                Self {
                    accent: Color::Rgb(136, 192, 208),
                    panel_bg: Color::Rgb(46, 52, 64),
                    surface0: Color::Rgb(59, 66, 82),
                    surface1: Color::Rgb(67, 76, 94),
                    surface_dim: Color::Rgb(46, 52, 64),
                    overlay0: Color::Rgb(76, 86, 106),
                    overlay1: Color::Rgb(100, 110, 130),
                    text: Color::Rgb(236, 239, 244),
                    subtext0: Color::Rgb(216, 222, 233),
                    mauve: Color::Rgb(180, 142, 173),
                    green: Color::Rgb(163, 190, 140),
                    yellow: Color::Rgb(235, 203, 139),
                    red: Color::Rgb(191, 97, 106),
                    blue: Color::Rgb(129, 161, 193),
                    teal: Color::Rgb(143, 188, 187),
                    peach: Color::Rgb(208, 135, 112),
                }
            }

            pub fn gruvbox() -> Self {
                Self {
                    accent: Color::Rgb(215, 153, 33),
                    panel_bg: Color::Rgb(40, 40, 40),
                    surface0: Color::Rgb(60, 56, 54),
                    surface1: Color::Rgb(80, 73, 69),
                    surface_dim: Color::Rgb(40, 40, 40),
                    overlay0: Color::Rgb(146, 131, 116),
                    overlay1: Color::Rgb(168, 153, 132),
                    text: Color::Rgb(235, 219, 178),
                    subtext0: Color::Rgb(213, 196, 161),
                    mauve: Color::Rgb(211, 134, 155),
                    green: Color::Rgb(184, 187, 38),
                    yellow: Color::Rgb(250, 189, 47),
                    red: Color::Rgb(251, 73, 52),
                    blue: Color::Rgb(131, 165, 152),
                    teal: Color::Rgb(142, 192, 124),
                    peach: Color::Rgb(254, 128, 25),
                }
            }

            pub fn one_dark() -> Self {
                Self {
                    accent: Color::Rgb(97, 175, 239),
                    panel_bg: Color::Rgb(40, 44, 52),
                    surface0: Color::Rgb(44, 49, 58),
                    surface1: Color::Rgb(62, 68, 81),
                    surface_dim: Color::Rgb(40, 44, 52),
                    overlay0: Color::Rgb(92, 99, 112),
                    overlay1: Color::Rgb(115, 122, 135),
                    text: Color::Rgb(171, 178, 191),
                    subtext0: Color::Rgb(150, 156, 168),
                    mauve: Color::Rgb(198, 120, 221),
                    green: Color::Rgb(152, 195, 121),
                    yellow: Color::Rgb(229, 192, 123),
                    red: Color::Rgb(224, 108, 117),
                    blue: Color::Rgb(97, 175, 239),
                    teal: Color::Rgb(86, 182, 194),
                    peach: Color::Rgb(209, 154, 102),
                }
            }

            pub fn solarized() -> Self {
                Self {
                    accent: Color::Rgb(38, 139, 210),
                    panel_bg: Color::Rgb(0, 43, 54),
                    surface0: Color::Rgb(7, 54, 66),
                    surface1: Color::Rgb(88, 110, 117),
                    surface_dim: Color::Rgb(0, 43, 54),
                    overlay0: Color::Rgb(88, 110, 117),
                    overlay1: Color::Rgb(101, 123, 131),
                    text: Color::Rgb(147, 161, 161),
                    subtext0: Color::Rgb(131, 148, 150),
                    mauve: Color::Rgb(211, 54, 130),
                    green: Color::Rgb(133, 153, 0),
                    yellow: Color::Rgb(181, 137, 0),
                    red: Color::Rgb(220, 50, 47),
                    blue: Color::Rgb(38, 139, 210),
                    teal: Color::Rgb(42, 161, 152),
                    peach: Color::Rgb(203, 75, 22),
                }
            }

            pub fn kanagawa() -> Self {
                Self {
                    accent: Color::Rgb(126, 156, 216),
                    panel_bg: Color::Rgb(31, 31, 40),
                    surface0: Color::Rgb(42, 42, 55),
                    surface1: Color::Rgb(54, 54, 70),
                    surface_dim: Color::Rgb(31, 31, 40),
                    overlay0: Color::Rgb(114, 113, 105),
                    overlay1: Color::Rgb(135, 134, 125),
                    text: Color::Rgb(220, 215, 186),
                    subtext0: Color::Rgb(200, 195, 170),
                    mauve: Color::Rgb(149, 127, 184),
                    green: Color::Rgb(118, 148, 106),
                    yellow: Color::Rgb(192, 163, 110),
                    red: Color::Rgb(195, 64, 67),
                    blue: Color::Rgb(126, 156, 216),
                    teal: Color::Rgb(127, 180, 202),
                    peach: Color::Rgb(255, 160, 102),
                }
            }

            pub fn rose_pine() -> Self {
                Self {
                    accent: Color::Rgb(196, 167, 231),
                    panel_bg: Color::Rgb(25, 23, 36),
                    surface0: Color::Rgb(31, 29, 46),
                    surface1: Color::Rgb(38, 35, 58),
                    surface_dim: Color::Rgb(25, 23, 36),
                    overlay0: Color::Rgb(110, 106, 134),
                    overlay1: Color::Rgb(144, 140, 170),
                    text: Color::Rgb(224, 222, 244),
                    subtext0: Color::Rgb(200, 197, 220),
                    mauve: Color::Rgb(196, 167, 231),
                    green: Color::Rgb(49, 116, 143),
                    yellow: Color::Rgb(246, 193, 119),
                    red: Color::Rgb(235, 111, 146),
                    blue: Color::Rgb(49, 116, 143),
                    teal: Color::Rgb(156, 207, 216),
                    peach: Color::Rgb(234, 154, 151),
                }
            }

            pub fn vesper() -> Self {
                Self {
                    accent: Color::Rgb(255, 199, 153),
                    panel_bg: Color::Rgb(26, 26, 26),
                    surface0: Color::Rgb(35, 35, 35),
                    surface1: Color::Rgb(40, 40, 40),
                    surface_dim: Color::Rgb(16, 16, 16),
                    overlay0: Color::Rgb(92, 92, 92),
                    overlay1: Color::Rgb(126, 126, 126),
                    text: Color::Rgb(255, 255, 255),
                    subtext0: Color::Rgb(160, 160, 160),
                    mauve: Color::Rgb(255, 209, 168),
                    green: Color::Rgb(153, 255, 228),
                    yellow: Color::Rgb(255, 199, 153),
                    red: Color::Rgb(255, 128, 128),
                    blue: Color::Rgb(176, 176, 176),
                    teal: Color::Rgb(102, 221, 204),
                    peach: Color::Rgb(255, 199, 153),
                }
            }

            pub fn from_name(name: &str) -> Option<Self> {
                match name.to_lowercase().replace([' ', '_'], "-").as_str() {
                    "catppuccin" => Some(Self::catppuccin()),
                    "terminal" => Some(Self::terminal()),
                    "tokyo-night" | "tokyonight" => Some(Self::tokyo_night()),
                    "dracula" => Some(Self::dracula()),
                    "nord" => Some(Self::nord()),
                    "gruvbox" => Some(Self::gruvbox()),
                    "one-dark" | "onedark" => Some(Self::one_dark()),
                    "solarized" => Some(Self::solarized()),
                    "kanagawa" => Some(Self::kanagawa()),
                    "rose-pine" | "rosepine" => Some(Self::rose_pine()),
                    "vesper" => Some(Self::vesper()),
                    _ => None,
                }
            }
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct StaticImagePlacement {
            pub formula: String,
            pub text_color_hex: String,
            pub area: ratatui::layout::Rect,
            pub grid_cols: u32,
            pub grid_rows: u32,
            pub viewport_col: i32,
            pub viewport_row: i32,
        }
    }
}

pub const THEME_NAMES: &[&str] = &[
    "catppuccin",
    "terminal",
    "tokyo-night",
    "dracula",
    "nord",
    "gruvbox",
    "one-dark",
    "solarized",
    "kanagawa",
    "rose-pine",
    "vesper",
];

mod kitty_graphics {
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct HostCellSize {
        pub width_px: u32,
        pub height_px: u32,
    }

    impl HostCellSize {
        pub fn from_terminal() -> Self {
            if let Ok(size) = crossterm::terminal::window_size() {
                if size.columns > 0 && size.rows > 0 && size.width > 0 && size.height > 0 {
                    return Self {
                        width_px: (size.width as u32 / size.columns as u32).max(1),
                        height_px: (size.height as u32 / size.rows as u32).max(1),
                    };
                }
            }
            Self::default()
        }

        pub fn is_known(self) -> bool {
            self.width_px > 0 && self.height_px > 0
        }
    }
}

pub mod math_compiler {
    pub fn lookup_math_cache(
        _formula: &str,
        _text_color_hex: &str,
    ) -> Option<(Vec<u8>, u32, u32, bool)> {
        None
    }

    pub fn enqueue_compile_job(_formula: String, _text_color_hex: String) {}
}

#[path = "../ui/markdown.rs"]
pub mod markdown;

fn open_url(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        #[cfg(target_os = "windows")]
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()?;
    }
    Ok(())
}

fn write_clipboard(text: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        use std::process::{Command, Stdio};
        if let Ok(mut child) = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            if let Ok(status) = child.wait() {
                return status.success();
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::io::Write;
        use std::process::{Command, Stdio};
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            if let Ok(mut child) = Command::new("wl-copy")
                .args(["--type", "text/plain;charset=utf-8"])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                if let Ok(status) = child.wait() {
                    if status.success() {
                        return true;
                    }
                }
            }
        }
        if std::env::var_os("DISPLAY").is_some() {
            if let Ok(mut child) = Command::new("xclip")
                .args(["-selection", "clipboard", "-in"])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                if let Ok(status) = child.wait() {
                    if status.success() {
                        return true;
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::io::Write;
        use std::process::{Command, Stdio};
        if let Ok(mut child) = Command::new("clip")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            if let Ok(status) = child.wait() {
                return status.success();
            }
        }
    }

    false
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut theme_name = "catppuccin".to_string();
    let mut file_path: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--theme" | "-t" => {
                if i + 1 < args.len() {
                    theme_name = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("error: --theme requires a value");
                    std::process::exit(1);
                }
            }
            "--list-themes" | "-l" => {
                println!("Available themes:");
                for theme in THEME_NAMES {
                    println!("  {}", theme);
                }
                std::process::exit(0);
            }
            "--help" | "-h" => {
                println!("herdr-md — Standalone TUI markdown renderer from herdr");
                println!();
                println!("Usage: herdr-md [options] [file]");
                println!();
                println!("Arguments:");
                println!("  [file]            The markdown file to render. Use '-' or omit to read from stdin.");
                println!();
                println!("Options:");
                println!("  -t, --theme <name>  Select color theme (default: catppuccin)");
                println!("  -l, --list-themes   List all available themes and exit");
                println!("  -h, --help          Show this help text");
                std::process::exit(0);
            }
            arg if arg.starts_with('-') => {
                eprintln!("error: unknown option: {}", arg);
                eprintln!("run 'herdr-md --help' for usage");
                std::process::exit(1);
            }
            arg => {
                if file_path.is_some() {
                    eprintln!("error: multiple input files specified");
                    std::process::exit(1);
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
                eprintln!("run 'herdr-md --help' for usage");
                std::process::exit(1);
            }
            use std::io::Read;
            let mut buf = String::new();
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
            use std::io::Read;
            let mut buf = String::new();
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

    let palette = app::state::Palette::from_name(&theme_name).unwrap_or_else(|| {
        eprintln!(
            "warning: theme '{}' not found, defaulting to catppuccin",
            theme_name
        );
        app::state::Palette::catppuccin()
    });

    let mut terminal = ratatui::init();
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;
    let mut scroll_y: u16 = 0;
    let mut scroll_x: u16 = 0;
    let mut status_message: Option<(String, std::time::Instant)> = None;

    let mut doc = markdown::MarkdownDocument::new();
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

        // Split layout: content and status bar
        let chunks = ratatui::layout::Layout::vertical([
            ratatui::layout::Constraint::Min(0),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(area);

        let content_area = chunks[0];
        let status_area = chunks[1];

        let text_width = content_area.width.saturating_sub(2) as usize; // padding + scrollbar space

        // Wrap the markdown document to compute width/scrollbar first
        let wrapped_temp = doc.wrap(
            text_width,
            scroll_x as usize,
            kitty_graphics::HostCellSize::default(),
            palette.text,
        );
        let max_x = wrapped_temp.max_scroll_x(content_area.width.saturating_sub(2));

        let has_h_scrollbar = max_x > 0;
        let text_height = if has_h_scrollbar {
            content_area.height.saturating_sub(1)
        } else {
            content_area.height
        };

        let text_area = ratatui::layout::Rect::new(
            content_area.x + 1,
            content_area.y,
            content_area.width.saturating_sub(2),
            text_height,
        );

        let wrapped = doc.wrap(
            text_width,
            scroll_x as usize,
            kitty_graphics::HostCellSize::default(),
            palette.text,
        );

        let max_y = wrapped.max_scroll_y(text_area.height);

        // Keep scroll within bounds
        scroll_y = scroll_y.min(max_y);
        scroll_x = scroll_x.min(max_x);

        // Render
        let draw_res = terminal.draw(|f| {
            // Render background
            f.render_widget(
                ratatui::widgets::Block::default()
                    .style(ratatui::style::Style::default().bg(palette.panel_bg)),
                area,
            );

            // Render Markdown text
            let lines = wrapped.lines();
            let paragraph = ratatui::widgets::Paragraph::new(lines.to_vec())
                .scroll((scroll_y, 0))
                .style(ratatui::style::Style::default().fg(palette.text));
            f.render_widget(paragraph, text_area);

            // Vertical scrollbar rendering
            if max_y > 0 {
                let track_height = text_area.height;
                let total_lines = lines.len();
                let thumb_height = (((track_height as f32) * (track_height as f32))
                    / (total_lines as f32))
                    .clamp(1.0, track_height as f32) as u16;
                let scrollable_height = track_height.saturating_sub(thumb_height);
                let thumb_y = if max_y > 0 {
                    ((scroll_y as f32 / max_y as f32) * (scrollable_height as f32)) as u16
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
                    let cell_area = ratatui::layout::Rect::new(
                        content_area.x + content_area.width - 1,
                        content_area.y + y,
                        1,
                        1,
                    );
                    f.render_widget(
                        ratatui::widgets::Paragraph::new(char_to_draw).style(style),
                        cell_area,
                    );
                }
            }

            // Horizontal scrollbar rendering
            if max_x > 0 {
                let track_width = content_area.width.saturating_sub(2);
                let max_width = wrapped.lines().iter().map(|l| l.width()).max().unwrap_or(0);
                let thumb_width = (((track_width as f32) * (track_width as f32))
                    / (max_width as f32))
                    .clamp(1.0, track_width as f32) as u16;
                let scrollable_width = track_width.saturating_sub(thumb_width);
                let thumb_x = if max_x > 0 {
                    ((scroll_x as f32 / max_x as f32) * (scrollable_width as f32)) as u16
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
                    let cell_area = ratatui::layout::Rect::new(
                        content_area.x + x + 1,
                        content_area.y + content_area.height - 1,
                        1,
                        1,
                    );
                    f.render_widget(
                        ratatui::widgets::Paragraph::new(char_to_draw).style(style),
                        cell_area,
                    );
                }
            }

            // Status Bar
            // Status Bar
            let percent = if max_y > 0 {
                (scroll_y as f32 / max_y as f32 * 100.0) as u32
            } else {
                100
            };

            let now = std::time::Instant::now();
            let left_status = if let Some((ref msg, timestamp)) = status_message {
                if now.duration_since(timestamp) < std::time::Duration::from_secs(2) {
                    format!(" {} ", msg)
                } else {
                    format!(" herdr-md: {} ", source_name)
                }
            } else {
                format!(" herdr-md: {} ", source_name)
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

        // Read event
        match crossterm::event::poll(std::time::Duration::from_millis(100)) {
            Ok(true) => {
                use crossterm::event::{Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};
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
                            let active_links = wrapped.active_hyperlinks(text_area, scroll_y);
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
                                    if write_clipboard(url) {
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

    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture)?;
    ratatui::restore();

    result
}
