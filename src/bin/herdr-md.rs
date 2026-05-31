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
    use std::fmt::Write as FmtWrite;
    use std::io::Write;

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

    #[derive(Debug, Default)]
    pub struct HostGraphicsCache {
        // image_id -> (formula, text_color_hex, width_px, height_px)
        pub uploaded: std::collections::HashMap<u32, (String, String, u32, u32)>,
        // (image_id, placement_id) -> (cols, rows, x, y)
        pub placements: std::collections::HashMap<(u32, u32), (u32, u32, u16, u16)>,
    }

    pub struct ClippedPlacement {
        pub x: u16,
        pub y: u16,
        pub cols: u32,
        pub rows: u32,
        pub source_x: u32,
        pub source_y: u32,
        pub source_width: u32,
        pub source_height: u32,
    }

    fn clip_static_placement(
        sp: &crate::app::state::StaticImagePlacement,
        cell_size: HostCellSize,
        image_width: u32,
        image_height: u32,
    ) -> Option<ClippedPlacement> {
        let area = sp.area;
        if area.width == 0 || area.height == 0 || sp.grid_cols == 0 || sp.grid_rows == 0 {
            return None;
        }

        let left_clip_cells = if sp.viewport_col < 0 {
            sp.viewport_col.saturating_neg() as u32
        } else {
            0
        };
        let top_clip_cells = if sp.viewport_row < 0 {
            sp.viewport_row.saturating_neg() as u32
        } else {
            0
        };

        let viewport_col = sp.viewport_col.max(0) as u32;
        let viewport_row = sp.viewport_row.max(0) as u32;

        if viewport_col >= area.width as u32 || viewport_row >= area.height as u32 {
            return None;
        }

        let visible_cols = sp
            .grid_cols
            .saturating_sub(left_clip_cells)
            .min(area.width as u32 - viewport_col);
        let visible_rows = sp
            .grid_rows
            .saturating_sub(top_clip_cells)
            .min(area.height as u32 - viewport_row);

        if visible_cols == 0 || visible_rows == 0 {
            return None;
        }

        let source_width = image_width;
        let source_height = image_height;
        let pixel_width = sp.grid_cols * cell_size.width_px;
        let pixel_height = sp.grid_rows * cell_size.height_px;

        let crop_left_px = left_clip_cells * cell_size.width_px;
        let crop_top_px = top_clip_cells * cell_size.height_px;
        let visible_width_px = visible_cols * cell_size.width_px;
        let visible_height_px = visible_rows * cell_size.height_px;

        let scale_pixels = |value: u32, source: u32, dest: u32| -> u32 {
            ((value as u64 * source as u64) / dest.max(1) as u64).min(u32::MAX as u64) as u32
        };

        let source_x = scale_pixels(crop_left_px, source_width, pixel_width);
        let source_y = scale_pixels(crop_top_px, source_height, pixel_height);
        let source_width = scale_pixels(visible_width_px, source_width, pixel_width)
            .max(1)
            .min(image_width.saturating_sub(source_x));
        let source_height = scale_pixels(visible_height_px, source_height, pixel_height)
            .max(1)
            .min(image_height.saturating_sub(source_y));

        if source_width == 0 || source_height == 0 {
            return None;
        }

        Some(ClippedPlacement {
            x: area.x + viewport_col as u16,
            y: area.y + viewport_row as u16,
            cols: visible_cols,
            rows: visible_rows,
            source_x,
            source_y,
            source_width,
            source_height,
        })
    }

    const KITTY_CHUNK_BYTES: usize = 3072;

    fn encode_kitty_data(out: &mut Vec<u8>, control: &str, data: &[u8]) {
        use base64::Engine;
        let mut chunks = data.chunks(KITTY_CHUNK_BYTES).peekable();
        let Some(first) = chunks.next() else {
            return;
        };
        let more = if chunks.peek().is_some() { 1 } else { 0 };
        let encoded = base64::engine::general_purpose::STANDARD.encode(first);
        let _ = write!(out, "\x1b_G{control},m={more};{encoded}\x1b\\");

        while let Some(chunk) = chunks.next() {
            let more = if chunks.peek().is_some() { 1 } else { 0 };
            let encoded = base64::engine::general_purpose::STANDARD.encode(chunk);
            let _ = write!(out, "\x1b_Gm={more};{encoded}\x1b\\");
        }
    }

    pub fn paint_math_placements(
        placements: &[crate::app::state::StaticImagePlacement],
        cell_size: HostCellSize,
        cache: &mut HostGraphicsCache,
    ) -> std::io::Result<()> {
        if !cell_size.is_known() || placements.is_empty() {
            return clear_all_placements(cache);
        }

        let mut out = Vec::new();
        let mut current_placements = std::collections::HashSet::new();

        for sp in placements {
            if let Some((png_bytes, w_px, h_px, failed)) =
                crate::math_compiler::lookup_math_cache(&sp.formula, &sp.text_color_hex)
            {
                if failed {
                    continue;
                }

                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                std::hash::Hash::hash(&sp.formula, &mut hasher);
                let formula_hash = std::hash::Hasher::finish(&hasher) as u32;

                let image_id = 900_000 + (formula_hash % 99_999);
                let placement_id = 900_000 + (formula_hash % 99_999);
                let placement_key = (image_id, placement_id);
                current_placements.insert(placement_key);

                let mut uploaded_width = 0;
                let mut uploaded_height = 0;

                let needs_upload = match cache.uploaded.get(&image_id) {
                    Some(&(ref f, ref c, w, h)) => {
                        uploaded_width = w;
                        uploaded_height = h;
                        f != &sp.formula || c != &sp.text_color_hex
                    }
                    None => true,
                };

                if needs_upload {
                    let grid_width_px = sp.grid_cols * cell_size.width_px;
                    let grid_height_px = sp.grid_rows * cell_size.height_px;

                    let (final_bytes, final_w, final_h) = if let Some(padded_bytes) =
                        crate::math_compiler::scale_and_pad_math_image(
                            &png_bytes,
                            w_px,
                            h_px,
                            grid_width_px,
                            grid_height_px,
                        ) {
                        (padded_bytes, grid_width_px, grid_height_px)
                    } else {
                        (png_bytes.clone(), w_px, h_px)
                    };

                    uploaded_width = final_w;
                    uploaded_height = final_h;

                    if !final_bytes.is_empty() {
                        if cache.uploaded.contains_key(&image_id) {
                            let _ = write!(&mut out, "\x1b_Ga=d,d=I,i={image_id},q=2;\x1b\\");
                        }

                        let control =
                            format!("a=t,t=d,f=100,s={},v={},i={image_id},q=2", final_w, final_h);
                        encode_kitty_data(&mut out, &control, &final_bytes);
                        cache.uploaded.insert(
                            image_id,
                            (
                                sp.formula.clone(),
                                sp.text_color_hex.clone(),
                                final_w,
                                final_h,
                            ),
                        );
                    }
                }

                if let Some(clipped) =
                    clip_static_placement(sp, cell_size, uploaded_width, uploaded_height)
                {
                    let placement_val = (clipped.cols, clipped.rows, clipped.x, clipped.y);
                    let needs_display = match cache.placements.get(&placement_key) {
                        Some(&existing) => existing != placement_val,
                        None => true,
                    };

                    if needs_display {
                        let z = 10;
                        let _ = write!(&mut out, "\x1b[{};{}H", clipped.y + 1, clipped.x + 1);
                        let mut control = format!(
                            "a=p,i={image_id},p={placement_id},c={},r={},z={z},C=1,q=2",
                            clipped.cols, clipped.rows,
                        );
                        if clipped.source_x > 0 {
                            let _ = write!(control, ",x={}", clipped.source_x);
                        }
                        if clipped.source_y > 0 {
                            let _ = write!(control, ",y={}", clipped.source_y);
                        }
                        if clipped.source_width > 0 {
                            let _ = write!(control, ",w={}", clipped.source_width);
                        }
                        if clipped.source_height > 0 {
                            let _ = write!(control, ",h={}", clipped.source_height);
                        }
                        let _ = write!(&mut out, "\x1b_G{control};\x1b\\");

                        cache.placements.insert(placement_key, placement_val);
                    }
                }
            } else {
                crate::math_compiler::enqueue_compile_job(
                    sp.formula.clone(),
                    sp.text_color_hex.clone(),
                );
            }
        }

        let mut stale_placements = Vec::new();
        for &key in cache.placements.keys() {
            if !current_placements.contains(&key) {
                stale_placements.push(key);
            }
        }
        for (img_id, plc_id) in stale_placements {
            let _ = write!(&mut out, "\x1b_Ga=d,d=i,i={img_id},p={plc_id},q=2;\x1b\\");
            cache.placements.remove(&(img_id, plc_id));
        }

        if !out.is_empty() {
            let mut framed = Vec::with_capacity(out.len() + 8);
            framed.extend_from_slice(b"\x1b7");
            framed.extend_from_slice(&out);
            framed.extend_from_slice(b"\x1b8");

            let mut stdout = std::io::stdout().lock();
            stdout.write_all(&framed)?;
            stdout.flush()?;
        }

        Ok(())
    }

    pub fn clear_all_placements(cache: &mut HostGraphicsCache) -> std::io::Result<()> {
        let mut out = Vec::new();
        for &id in cache.uploaded.keys() {
            let _ = write!(&mut out, "\x1b_Ga=d,d=I,i={id},q=2;\x1b\\");
        }
        cache.uploaded.clear();
        cache.placements.clear();

        if !out.is_empty() {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(&out)?;
            stdout.flush()?;
        }
        Ok(())
    }

    pub fn detect_support() -> bool {
        let Ok(size) = crossterm::terminal::window_size() else {
            return false;
        };
        if size.columns == 0 || size.rows == 0 || size.width == 0 || size.height == 0 {
            return false;
        }

        let term_program = std::env::var("TERM_PROGRAM").ok();
        let term_program = term_program.as_deref();
        let term = std::env::var("TERM").ok();
        let term = term.as_deref();

        if term_program == Some("ghostty") || std::env::var_os("GHOSTTY_RESOURCES_DIR").is_some() {
            return true;
        }
        if term_program == Some("WezTerm") || std::env::var_os("WEZTERM_PANE").is_some() {
            return true;
        }
        if term_program == Some("kitty") || term == Some("xterm-kitty") {
            return true;
        }
        if term_program == Some("Konsole") || std::env::var_os("KONSOLE_VERSION").is_some() {
            return true;
        }
        if term_program == Some("foot") || term == Some("foot") {
            return true;
        }
        if term_program == Some("rio") {
            return true;
        }
        if std::env::var_os("TMUX").is_some() {
            return true;
        }

        if term_program == Some("Apple_Terminal") || term_program == Some("vscode") {
            return false;
        }
        if std::env::var_os("GNOME_TERMINAL_SCREEN").is_some()
            || std::env::var_os("GNOME_TERMINAL_SERVICE").is_some()
        {
            return false;
        }

        false
    }
}

#[path = "../math_compiler.rs"]
#[allow(dead_code)]
pub mod math_compiler;

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

    let mut graphics_cache = kitty_graphics::HostGraphicsCache::default();
    let kitty_graphics_enabled = kitty_graphics::detect_support();

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

        let cell_size = if kitty_graphics_enabled {
            kitty_graphics::HostCellSize::from_terminal()
        } else {
            kitty_graphics::HostCellSize::default()
        };

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
        let wrapped_temp = doc.wrap(text_width, scroll_x as usize, cell_size, palette.text);
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

        let wrapped = doc.wrap(text_width, scroll_x as usize, cell_size, palette.text);

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

        if kitty_graphics_enabled && cell_size.is_known() {
            let mut static_placements = Vec::new();
            wrapped.push_image_placements(&mut static_placements, text_area, scroll_y);
            let _ = kitty_graphics::paint_math_placements(
                &static_placements,
                cell_size,
                &mut graphics_cache,
            );
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

    if kitty_graphics_enabled {
        let _ = kitty_graphics::clear_all_placements(&mut graphics_cache);
    }
    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture)?;
    ratatui::restore();

    result
}
