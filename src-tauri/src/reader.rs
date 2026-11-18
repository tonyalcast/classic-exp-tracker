use std::path::PathBuf;
use std::time::{Duration, Instant};

use capture::{GameWindow, RgbaImage, image::imageops};
use exp_core::{ExpReading, ExpTable};
use ocr::{Fonts, GlyphSet, Layout, LineRead, Rect, View, learn, scale};
use serde::Serialize;

use crate::state::unix_ms;

/// Failed reads in a row before the status bar is located again.
const RELOCATE_AFTER: u32 = 3;
/// A percent drop larger than this means a level-up.
const LEVEL_UP_DROP: f64 = 50.0;
/// Game level cap.
const MAX_LEVEL: u32 = 100;
/// Minimum time between full display-scale searches while the status bar is not found.
const SEARCH_EVERY: Duration = Duration::from_secs(10);

/// What the UI shows about the OCR pipeline.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReaderStatus {
    pub ok: bool,
    pub message: String,
    pub exp_text: Option<String>,
    pub read_ms: Option<f64>,
    pub learned: usize,
    pub at_ms: i64,
}

impl ReaderStatus {
    fn new(ok: bool, message: impl Into<String>, exp_text: Option<String>, t: Instant, learned: usize) -> Self {
        Self {
            ok,
            message: message.into(),
            exp_text,
            read_ms: Some(t.elapsed().as_secs_f64() * 1000.0),
            learned,
            at_ms: unix_ms(),
        }
    }
}

/// Successful EXP read with its raw text, or an error message with the text seen so far.
type ExpResult = Result<Option<(ExpReading, String)>, (String, Option<String>)>;

/// Character identity read once from the status bar.
#[derive(Debug, Clone)]
pub struct Identity {
    pub name: Option<String>,
    pub level: u32,
}

/// Captures the game window and turns the status bar into readings.
pub struct Reader {
    fonts: Fonts,
    table: ExpTable,
    dir: PathBuf,
    window: Option<GameWindow>,
    /// Size of the capture the layout was found in.
    dims: Option<(u32, u32)>,
    /// How game pixels map onto the capture (1:1, or stretched by OS display scaling).
    view: View,
    layout: Option<Layout>,
    next_search: Option<Instant>,
    last_hash: Option<u64>,
    level: Option<u32>,
    last_pct: Option<f64>,
    fails: u32,
    learned: usize,
}

impl Reader {
    /// Loads built-in fonts plus glyphs learned earlier from `dir`.
    pub fn new(dir: PathBuf, table: ExpTable) -> Self {
        let mut fonts = Fonts::builtin();
        let mut learned = 0;
        for font in [Font::Status, Font::Level] {
            if let Some(extra) = load(&dir, font) {
                learned += extra.len();
                font.set(&mut fonts).merge(&extra);
            }
        }
        Self {
            fonts,
            table,
            dir,
            window: None,
            dims: None,
            view: View::NATIVE,
            layout: None,
            next_search: None,
            last_hash: None,
            level: None,
            last_pct: None,
            fails: 0,
            learned,
        }
    }

    pub fn level(&self) -> Option<u32> {
        self.level
    }

    /// Forgets per-session state; identity is kept.
    pub fn reset(&mut self) {
        self.last_hash = None;
        self.last_pct = None;
        self.fails = 0;
    }

    /// Forgets the identity so the next tick reads it again.
    pub fn forget_identity(&mut self) {
        self.level = None;
        self.layout = None;
        self.reset();
    }

    /// Reads name and level once: level from its digits or, failing that, from the EXP table.
    pub fn identify(&mut self) -> Result<(Identity, ReaderStatus), ReaderStatus> {
        let t = Instant::now();
        let fail = |r: &Self, m: String, text: Option<String>| ReaderStatus::new(false, m, text, t, r.learned);
        let (img, layout) = self.frame().map_err(|m| fail(self, m, None))?;

        let (level_line, mut level) = ocr::read_level(&img, &layout, &self.fonts);
        let (mut exp_line, mut exp) = ocr::read_exp(&img, &layout, &self.fonts);
        if exp.is_none()
            && let Some(required) = level.and_then(|l| self.table.required(l))
            && self.infer_exp(&exp_line, required)
        {
            (exp_line, exp) = ocr::read_exp(&img, &layout, &self.fonts);
        }
        if level.is_none() {
            level = exp.and_then(|(e, p)| self.table.level_for(e, p));
            if let Some(l) = level {
                self.learn(Font::Level, &level_line, &l.to_string());
            }
        }
        let Some(level) = level else {
            return Err(fail(self, "Could not read the level yet".into(), Some(exp_line.text)));
        };

        let name = sysocr::read_text(&crop(&img, layout.name)).ok().filter(|n| !n.is_empty());
        self.level = Some(level);
        self.last_pct = exp.map(|(_, p)| p);
        let who = name.as_deref().unwrap_or("character");
        let status = ReaderStatus::new(true, format!("Identified {who} · Lv {level}"), Some(exp_line.text), t, self.learned);
        Ok((Identity { name, level }, status))
    }

    /// Reads the EXP line; (None, None) when it did not change.
    pub fn step(&mut self) -> (Option<ExpReading>, Option<ReaderStatus>) {
        let t = Instant::now();
        match self.read_exp() {
            Ok(None) => (None, None),
            Ok(Some((reading, text))) => {
                self.fails = 0;
                (Some(reading), Some(ReaderStatus::new(true, "Reading EXP", Some(text), t, self.learned)))
            }
            Err((message, text)) => {
                self.fails += 1;
                if self.fails >= RELOCATE_AFTER {
                    self.layout = None;
                    self.last_hash = None;
                }
                (None, Some(ReaderStatus::new(false, message, text, t, self.learned)))
            }
        }
    }

    fn read_exp(&mut self) -> ExpResult {
        let current = self.level.ok_or(("Level unknown".to_string(), None))?;
        let (img, layout) = self.frame().map_err(|m| (m, None))?;
        let hash = hash_rect(&img, layout.exp);
        if self.last_hash == Some(hash) {
            return Ok(None);
        }
        self.last_hash = Some(hash);

        let (mut line, mut exp) = ocr::read_exp(&img, &layout, &self.fonts);
        if exp.is_none() {
            // A level-up may have just happened, so try both candidates.
            for lv in [current, current + 1] {
                if let Some(required) = self.table.required(lv)
                    && self.infer_exp(&line, required)
                {
                    (line, exp) = ocr::read_exp(&img, &layout, &self.fonts);
                    break;
                }
            }
        }
        let Some((value, percent)) = exp else {
            return Err(("Could not read EXP".into(), Some(line.text)));
        };

        let leveled = self.last_pct.is_some_and(|p| percent + LEVEL_UP_DROP < p);
        let by_drop = if leveled { (current + 1).min(MAX_LEVEL) } else { current };
        let level = self.table.level_for(value, percent).unwrap_or(by_drop);
        self.level = Some(level);
        self.last_pct = Some(percent);
        Ok(Some((ExpReading::new(level, value, percent), line.text)))
    }

    // Captures the game window, keeps the layout fresh and returns the status bar band at 1:1.
    fn frame(&mut self) -> Result<(RgbaImage, Layout), String> {
        if self.window.is_none() {
            self.window = GameWindow::find().map_err(|e| e.to_string())?;
        }
        let win = self.window.as_ref().ok_or("Game window not found")?;
        let img = match win.capture() {
            Ok(img) => img,
            Err(e) => {
                self.window = None;
                return Err(format!("Capture failed: {e}"));
            }
        };
        if self.dims != Some(img.dimensions()) {
            self.dims = Some(img.dimensions());
            self.layout = None;
        }
        if self.layout.is_none() {
            self.layout = self.find_layout(&img);
            self.last_hash = None;
        }
        let Some(layout) = self.layout else {
            return Err(not_visible(&img));
        };
        Ok((self.view.band(&img), layout))
    }

    // Known view first, then 1:1; the full scale search runs at most every SEARCH_EVERY.
    fn find_layout(&mut self, img: &RgbaImage) -> Option<Layout> {
        for view in [self.view, View::NATIVE] {
            if let Some(layout) = scale::locate_in(img, &self.fonts.labels, view) {
                self.view = view;
                self.next_search = None;
                return Some(layout);
            }
        }
        let now = Instant::now();
        if self.next_search.is_some_and(|t| now < t) {
            return None;
        }
        self.next_search = Some(now + SEARCH_EVERY);
        let (view, layout) = scale::detect(img, &self.fonts)?;
        self.view = view;
        self.next_search = None;
        Some(layout)
    }

    fn infer_exp(&mut self, line: &LineRead, required: u64) -> bool {
        learn::infer_exp_digits(line, required).is_some_and(|guess| self.learn(Font::Status, line, &guess))
    }

    // Adds glyphs for the unknown positions and persists them; true when something was learned.
    fn learn(&mut self, font: Font, line: &LineRead, expected: &str) -> bool {
        let added = learn::learn_expected(line, expected, font.set(&mut self.fonts));
        if added == 0 {
            return false;
        }
        self.learned += added;
        let mut saved = load(&self.dir, font).unwrap_or_default();
        learn::learn_expected(line, expected, &mut saved);
        let _ = std::fs::create_dir_all(&self.dir);
        if let Err(e) = std::fs::write(self.dir.join(font.file()), saved.to_text()) {
            eprintln!("could not save learned glyphs: {e}");
        }
        true
    }
}

#[derive(Clone, Copy)]
enum Font {
    Status,
    Level,
}

impl Font {
    fn set(self, fonts: &mut Fonts) -> &mut GlyphSet {
        match self {
            Font::Status => &mut fonts.status,
            Font::Level => &mut fonts.level,
        }
    }

    fn file(self) -> &'static str {
        match self {
            Font::Status => "status.learned.glyphs",
            Font::Level => "level.learned.glyphs",
        }
    }
}

fn load(dir: &std::path::Path, font: Font) -> Option<GlyphSet> {
    std::fs::read_to_string(dir.join(font.file()))
        .ok()
        .and_then(|t| GlyphSet::from_text(&t).ok())
}

// Error for a capture without a status bar, with a hint when the capture is blank.
fn not_visible(img: &RgbaImage) -> String {
    let (w, h) = img.dimensions();
    let step = (w.max(h) / 64).max(1) as usize;
    let blank = img.pixels().step_by(step).all(|p| p.0[..3].iter().all(|&c| c < 16));
    if blank {
        format!("Game capture is blank ({w}×{h}); use windowed or borderless mode")
    } else {
        format!("Status bar not visible in the {w}×{h} capture")
    }
}

fn crop(img: &RgbaImage, r: Rect) -> RgbaImage {
    let r = r.clamp(img.width(), img.height());
    imageops::crop_imm(img, r.x, r.y, r.w, r.h).to_image()
}

// FNV-1a over the pixels of a region.
fn hash_rect(img: &RgbaImage, r: Rect) -> u64 {
    let r = r.clamp(img.width(), img.height());
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for y in r.y..r.y + r.h {
        for x in r.x..r.x + r.w {
            for &b in &img.get_pixel(x, y).0 {
                h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
            }
        }
    }
    h
}
