use comemo::Prehashed;
use once_cell::sync::Lazy;
use resvg::usvg::{self, Options, PostProcessingSteps, Tree, TreeParsing, TreePostProc};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::mpsc;
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::{Library, World};

pub struct MathCacheEntry {
    pub png_bytes: Vec<u8>,
    pub width_px: u32,
    pub height_px: u32,
    pub last_accessed: Instant,
    pub failed: bool,
}

static MATH_CACHE: Lazy<Mutex<HashMap<String, MathCacheEntry>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
type CompileJobSender = mpsc::UnboundedSender<(String, String)>;
static COMPILE_TX: Lazy<Mutex<Option<CompileJobSender>>> = Lazy::new(|| Mutex::new(None));
static REDRAW_NOTIFY: Lazy<Mutex<Option<Arc<tokio::sync::Notify>>>> =
    Lazy::new(|| Mutex::new(None));
static REDRAW_DIRTY: Lazy<Mutex<Option<Arc<AtomicBool>>>> = Lazy::new(|| Mutex::new(None));

pub(crate) fn init_redraw_notifier(notify: Arc<tokio::sync::Notify>, dirty: Arc<AtomicBool>) {
    if let Ok(mut lock) = REDRAW_NOTIFY.lock() {
        *lock = Some(notify);
    }
    if let Ok(mut lock) = REDRAW_DIRTY.lock() {
        *lock = Some(dirty);
    }
}

fn trigger_redraw() {
    if let Ok(lock) = REDRAW_DIRTY.lock() {
        if let Some(dirty) = &*lock {
            dirty.store(true, Ordering::Release);
        }
    }
    if let Ok(lock) = REDRAW_NOTIFY.lock() {
        if let Some(notify) = &*lock {
            notify.notify_one();
        }
    }
}

pub(crate) fn lookup_math_cache(
    formula: &str,
    text_color_hex: &str,
) -> Option<(Vec<u8>, u32, u32, bool)> {
    let key = format!("{}:{}", text_color_hex, formula);
    if let Ok(mut cache) = MATH_CACHE.lock() {
        if let Some(entry) = cache.get_mut(&key) {
            entry.last_accessed = Instant::now();
            return Some((
                entry.png_bytes.clone(),
                entry.width_px,
                entry.height_px,
                entry.failed,
            ));
        }
    }
    None
}

pub(crate) fn enqueue_compile_job(formula: String, text_color_hex: String) {
    if lookup_math_cache(&formula, &text_color_hex).is_some() {
        return;
    }

    let mut tx_lock = COMPILE_TX.lock().unwrap();
    if tx_lock.is_none() {
        let (tx, rx) = mpsc::unbounded_channel::<(String, String)>();
        *tx_lock = Some(tx);
        std::thread::spawn(move || {
            run_compiler_worker(rx);
        });
    }

    if let Some(tx) = &*tx_lock {
        let _ = tx.send((formula, text_color_hex));
    }
}

fn run_compiler_worker(mut rx: mpsc::UnboundedReceiver<(String, String)>) {
    let mut book = FontBook::new();
    let mut fonts = Vec::new();
    for data in typst_assets::fonts() {
        let buffer = Bytes::from_static(data);
        for font in Font::iter(buffer) {
            book.push(font.info().clone());
            fonts.push(font);
        }
    }
    let font_db = usvg::fontdb::Database::new();

    let library = Prehashed::new(Library::default());
    let book = Prehashed::new(book);

    while let Some((formula, text_color_hex)) = rx.blocking_recv() {
        match compile_formula(&formula, &text_color_hex, &library, &book, &fonts, &font_db) {
            Ok((png_bytes, width_px, height_px)) => {
                insert_into_cache(
                    formula,
                    text_color_hex,
                    png_bytes,
                    width_px,
                    height_px,
                    false,
                );
                trigger_redraw();
            }
            Err(err) => {
                tracing::error!(formula = %formula, err = ?err, "Failed to compile LaTeX math formula");
                insert_into_cache(formula, text_color_hex, Vec::new(), 0, 0, true);
                trigger_redraw();
            }
        }
    }
}

fn insert_into_cache(
    formula: String,
    text_color_hex: String,
    png_bytes: Vec<u8>,
    width_px: u32,
    height_px: u32,
    failed: bool,
) {
    let key = format!("{}:{}", text_color_hex, formula);
    if let Ok(mut cache) = MATH_CACHE.lock() {
        if cache.len() >= 500 {
            let mut lru_key: Option<String> = None;
            let mut oldest_accessed = Instant::now();
            for (key, entry) in cache.iter() {
                if entry.last_accessed < oldest_accessed {
                    oldest_accessed = entry.last_accessed;
                    lru_key = Some(key.clone());
                }
            }
            if let Some(key) = lru_key {
                cache.remove(&key);
            }
        }
        cache.insert(
            key,
            MathCacheEntry {
                png_bytes,
                width_px,
                height_px,
                last_accessed: Instant::now(),
                failed,
            },
        );
    }
}

fn compile_formula(
    formula: &str,
    text_color_hex: &str,
    library: &Prehashed<Library>,
    book: &Prehashed<FontBook>,
    fonts: &[Font],
    font_db: &usvg::fontdb::Database,
) -> Result<(Vec<u8>, u32, u32), String> {
    let typst_math =
        mitex::convert_math(formula, None).map_err(|e| format!("mitex conversion failed: {e}"))?;

    let typst_source = format!(
        r#"
#set page(width: auto, height: auto, margin: 0pt, fill: none)
#set text(fill: rgb("{}"), size: 14pt)
#let mitexfrac(num, den) = $frac(#num, #den)$
#let mitexsqrt(arg) = $sqrt(#arg)$
$ {} $
"#,
        text_color_hex, typst_math
    );

    let main_id = FileId::new(None, typst::syntax::VirtualPath::new("/main.typ"));
    let world = MathWorld {
        library: library.clone(),
        book: book.clone(),
        fonts: fonts.to_vec(),
        main_id,
        source: Source::new(main_id, typst_source),
    };

    let mut tracer = typst::eval::Tracer::new();
    let document = typst::compile(&world, &mut tracer)
        .map_err(|diags| format!("typst compilation failed: {diags:?}"))?;

    if document.pages.is_empty() {
        return Err("typst compiled to 0 pages".to_string());
    }

    let frame = &document.pages[0].frame;
    let svg_string = typst_svg::svg(frame);

    let opt = Options::default();
    let mut rtree =
        Tree::from_str(&svg_string, &opt).map_err(|e| format!("usvg tree parse failed: {e:?}"))?;

    let steps = PostProcessingSteps {
        convert_text_into_paths: true,
    };
    rtree.postprocess(steps, font_db);

    let pixmap_size = rtree.size.to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height())
        .ok_or_else(|| "Failed to create tiny-skia Pixmap".to_string())?;

    resvg::render(
        &rtree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );

    let png_bytes = pixmap
        .encode_png()
        .map_err(|e| format!("Failed to encode PNG: {e}"))?;

    Ok((png_bytes, pixmap_size.width(), pixmap_size.height()))
}

struct MathWorld {
    library: Prehashed<Library>,
    book: Prehashed<FontBook>,
    fonts: Vec<Font>,
    main_id: FileId,
    source: Source,
}

impl World for MathWorld {
    fn library(&self) -> &Prehashed<Library> {
        &self.library
    }

    fn book(&self) -> &Prehashed<FontBook> {
        &self.book
    }

    fn main(&self) -> Source {
        self.source.clone()
    }

    fn source(&self, id: FileId) -> Result<Source, typst::diag::FileError> {
        if id == self.main_id {
            Ok(self.source.clone())
        } else {
            Err(typst::diag::FileError::NotFound(
                id.vpath().as_rootless_path().to_path_buf(),
            ))
        }
    }

    fn file(&self, id: FileId) -> Result<Bytes, typst::diag::FileError> {
        Err(typst::diag::FileError::NotFound(
            id.vpath().as_rootless_path().to_path_buf(),
        ))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        None
    }
}
