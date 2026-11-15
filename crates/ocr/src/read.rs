use image::RgbaImage;

use crate::glyph::{Glyph, segment};
use crate::mask::{Ink, Mask, Rect};
use crate::set::GlyphSet;

/// Placeholder for glyphs that match no template.
pub const UNKNOWN: char = '?';

/// Recognized text of one line plus its raw glyphs.
#[derive(Debug, Clone, Default)]
pub struct LineRead {
    pub text: String,
    pub glyphs: Vec<Glyph>,
    /// Indexes into `glyphs` (and chars of `text`) that matched no template.
    pub unknown: Vec<usize>,
}

impl LineRead {
    pub fn is_complete(&self) -> bool {
        self.unknown.is_empty()
    }
}

/// Reads one line of text inside `rect` using `ink` and `set`.
pub fn read_line(img: &RgbaImage, rect: Rect, ink: Ink, set: &GlyphSet) -> LineRead {
    recognize(segment(&Mask::from_image(img, rect, ink)), set)
}

/// Turns segmented glyphs into text.
pub fn recognize(glyphs: Vec<Glyph>, set: &GlyphSet) -> LineRead {
    let mut text = String::with_capacity(glyphs.len());
    let mut unknown = Vec::new();
    for (i, g) in glyphs.iter().enumerate() {
        match set.lookup(g) {
            Some((c, _)) => text.push(c),
            None => {
                text.push(UNKNOWN);
                unknown.push(i);
            }
        }
    }
    LineRead { text, glyphs, unknown }
}
