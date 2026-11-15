//! Glyph template OCR for the MapleStory Classic status bar.

pub mod glyph;
pub mod learn;
pub mod locate;
pub mod mask;
pub mod parse;
pub mod read;
pub mod set;

use image::RgbaImage;

pub use glyph::Glyph;
pub use locate::{Labels, Layout};
pub use mask::{Ink, Mask, Rect};
pub use read::{LineRead, UNKNOWN};
pub use set::GlyphSet;

const STATUS_GLYPHS: &str = include_str!("../assets/status.glyphs");
const LEVEL_GLYPHS: &str = include_str!("../assets/level.glyphs");
const LABELS: &str = include_str!("../assets/labels.glyphs");

/// Fonts that ship with the app; extra learned glyphs are merged on top.
#[derive(Debug, Clone)]
pub struct Fonts {
    pub status: GlyphSet,
    pub level: GlyphSet,
    pub labels: Labels,
}

impl Fonts {
    pub fn builtin() -> Self {
        let labels = GlyphSet::from_text(LABELS).expect("valid labels asset");
        let label = |c| {
            labels
                .templates()
                .iter()
                .find(|t| t.ch == c)
                .map(|t| t.mask.clone())
                .expect("label present")
        };
        Self {
            status: GlyphSet::from_text(STATUS_GLYPHS).expect("valid status asset"),
            level: GlyphSet::from_text(LEVEL_GLYPHS).expect("valid level asset"),
            labels: Labels { exp: label('E'), level: label('L') },
        }
    }
}

/// Raw result of reading one frame.
#[derive(Debug, Clone)]
pub struct FrameRead {
    pub exp_line: LineRead,
    pub level_line: LineRead,
    pub exp: Option<(u64, f64)>,
    pub level: Option<u32>,
}

/// Reads only the EXP line.
pub fn read_exp(img: &RgbaImage, layout: &Layout, fonts: &Fonts) -> (LineRead, Option<(u64, f64)>) {
    let line = read::read_line(img, layout.exp, Ink::Status, &fonts.status);
    let exp = parse::parse_exp(&line.text);
    (line, exp)
}

/// Reads only the level digits.
pub fn read_level(img: &RgbaImage, layout: &Layout, fonts: &Fonts) -> (LineRead, Option<u32>) {
    let line = read::read_line(img, layout.level, Ink::White, &fonts.level);
    let level = parse::parse_level(&line.text);
    (line, level)
}

/// Reads EXP and level from a full window capture with a known layout.
pub fn read_frame(img: &RgbaImage, layout: &Layout, fonts: &Fonts) -> FrameRead {
    let exp_line = read::read_line(img, layout.exp, Ink::Status, &fonts.status);
    let level_line = read::read_line(img, layout.level, Ink::White, &fonts.level);
    FrameRead {
        exp: parse::parse_exp(&exp_line.text),
        level: parse::parse_level(&level_line.text),
        exp_line,
        level_line,
    }
}
