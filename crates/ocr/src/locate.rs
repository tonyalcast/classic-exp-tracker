use image::RgbaImage;

use crate::mask::{Ink, Mask, Rect};

/// Height of the bottom band searched for the status bar labels.
pub const BAND: u32 = 90;
/// Fraction of label pixels allowed to differ on a 1:1 capture.
pub const MAX_MISMATCH: f64 = 0.04;
/// EXP value spans from the end of the `EXP.` label to this many px right of its start.
const EXP_SPAN: u32 = 128;
/// Vertical padding around label rows for value regions.
const PAD_Y: u32 = 3;
/// Horizontal search distance for the orange level tiles after the `LV.` label.
const LEVEL_SEARCH: u32 = 60;
/// Name line: offset from the level tiles' right edge and from the `LV.` label top, and size.
const NAME_DX: u32 = 11;
const NAME_DY: u32 = 5;
const NAME_W: u32 = 110;
const NAME_H: u32 = 15;

/// Regions of the status bar fields inside a captured window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub image_w: u32,
    pub image_h: u32,
    pub exp_label: Rect,
    pub level_label: Rect,
    pub exp: Rect,
    pub level: Rect,
    /// Character name (second line next to the level).
    pub name: Rect,
}

/// Status bar label shapes used as anchors.
#[derive(Debug, Clone)]
pub struct Labels {
    pub exp: Mask,
    pub level: Mask,
}

/// Finds the status bar fields; None when the labels are not visible.
pub fn locate(img: &RgbaImage, labels: &Labels) -> Option<Layout> {
    locate_scored(img, labels, MAX_MISMATCH).map(|(l, _)| l)
}

/// Like [`locate`] with a custom label tolerance; also returns the labels' mismatch fraction.
pub fn locate_scored(img: &RgbaImage, labels: &Labels, max_mismatch: f64) -> Option<(Layout, f64)> {
    let band_y = img.height().saturating_sub(BAND);
    let band = Mask::from_image(img, Rect::new(0, band_y, img.width(), BAND), Ink::White);
    let (ex, ey, ed) = find_scored(&band, &labels.exp, max_mismatch)?;
    let (lx, ly, ld) = find_scored(&band, &labels.level, max_mismatch)?;
    let area = labels.exp.w * labels.exp.h + labels.level.w * labels.level.h;
    let score = (ed + ld) as f64 / f64::from(area);
    let exp_label = Rect::new(ex, band_y + ey, labels.exp.w, labels.exp.h);
    let level_label = Rect::new(lx, band_y + ly, labels.level.w, labels.level.h);

    let exp_x = exp_label.x + exp_label.w + 1;
    let exp = Rect::new(
        exp_x,
        exp_label.y.saturating_sub(PAD_Y),
        (exp_label.x + EXP_SPAN).saturating_sub(exp_x),
        exp_label.h + 2 * PAD_Y,
    )
    .clamp(img.width(), img.height());

    let search = Rect::new(
        level_label.x + level_label.w,
        level_label.y.saturating_sub(PAD_Y),
        LEVEL_SEARCH,
        level_label.h + 2 * PAD_Y,
    )
    .clamp(img.width(), img.height());
    let level = orange_span(img, search)?;
    let name = Rect::new(level.x + level.w + NAME_DX, level_label.y + NAME_DY, NAME_W, NAME_H)
        .clamp(img.width(), img.height());

    let layout = Layout { image_w: img.width(), image_h: img.height(), exp_label, level_label, exp, level, name };
    Some((layout, score))
}

/// Best position of `needle` inside `hay`, if close enough.
pub fn find(hay: &Mask, needle: &Mask) -> Option<(u32, u32)> {
    find_scored(hay, needle, MAX_MISMATCH).map(|(x, y, _)| (x, y))
}

/// Best position of `needle` inside `hay` and its differing pixel count, within `max_mismatch`.
pub fn find_scored(hay: &Mask, needle: &Mask, max_mismatch: f64) -> Option<(u32, u32, usize)> {
    if needle.w > hay.w || needle.h > hay.h || needle.ink_count() == 0 {
        return None;
    }
    let ink: Vec<(u32, u32)> = (0..needle.h)
        .flat_map(|y| (0..needle.w).map(move |x| (x, y)))
        .filter(|&(x, y)| needle.get(x, y))
        .collect();
    let limit = ((needle.w * needle.h) as f64 * max_mismatch).floor() as usize;
    let mut best: Option<(usize, u32, u32)> = None;
    for oy in 0..=hay.h - needle.h {
        for ox in 0..=hay.w - needle.w {
            // Cheap rejection on ink pixels first, full compare only for candidates.
            let mut misses = 0;
            for &(x, y) in &ink {
                misses += usize::from(!hay.get(ox + x, oy + y));
                if misses > limit {
                    break;
                }
            }
            if misses > limit {
                continue;
            }
            let mut diff = 0;
            for y in 0..needle.h {
                for x in 0..needle.w {
                    diff += usize::from(needle.get(x, y) != hay.get(ox + x, oy + y));
                }
            }
            if diff <= limit && best.is_none_or(|(d, _, _)| diff < d) {
                best = Some((diff, ox, oy));
            }
        }
    }
    best.map(|(d, x, y)| (x, y, d))
}

// Bounding box of the orange tile pixels inside `search`.
fn orange_span(img: &RgbaImage, search: Rect) -> Option<Rect> {
    let m = Mask::from_image(img, search, Ink::Orange);
    let x0 = (0..m.w).find(|&x| m.col_has_ink(x))?;
    let x1 = (0..m.w).rev().find(|&x| m.col_has_ink(x))?;
    let y0 = (0..m.h).find(|&y| m.row_has_ink(y))?;
    let y1 = (0..m.h).rev().find(|&y| m.row_has_ink(y))?;
    Some(Rect::new(search.x + x0, search.y + y0, x1 - x0 + 1, y1 - y0 + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_exact_pattern() {
        let hay = Mask::from_rows(&["......", "..##..", "..#...", "......"]).unwrap();
        let needle = Mask::from_rows(&["##", "#."]).unwrap();
        assert_eq!(find(&hay, &needle), Some((2, 1)));
    }

    #[test]
    fn rejects_missing_pattern() {
        let hay = Mask::from_rows(&["......", "......"]).unwrap();
        let needle = Mask::from_rows(&["##"]).unwrap();
        assert_eq!(find(&hay, &needle), None);
    }
}
