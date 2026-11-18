//! Undoes OS stretching of the game window, e.g. Windows scaling a DPI-unaware game to 125%.

use image::{Rgba, RgbaImage, imageops};

use crate::Fonts;
use crate::locate::{self, BAND, Labels, Layout};

/// Display scales tried when the status bar does not match 1:1 (Windows scaling steps).
pub const SCALES: [f32; 7] = [1.25, 1.5, 1.75, 2.0, 2.25, 2.5, 3.0];
/// Sub-pixel offsets per axis for the coarse pass and the fine pass.
const COARSE_PHASES: u32 = 2;
const FINE_PHASES: u32 = 8;
/// Label tolerance on a rebuilt band, which is never pixel exact.
const LOOSE_MISMATCH: f64 = 0.2;
/// Extra game pixels kept around the labels and EXP value in the fine pass.
const FINE_MARGIN: u32 = 16;
/// Contrast stretch that brings blurred ink back over the 1:1 color thresholds.
const LEVEL_LO: f32 = 40.0;
const LEVEL_HI: f32 = 175.0;

/// How the game pixels map onto a capture: scale plus sub-pixel offset in capture pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub scale: f32,
    pub dx: f32,
    pub dy: f32,
}

impl View {
    pub const NATIVE: Self = Self { scale: 1.0, dx: 0.0, dy: 0.0 };

    pub fn is_native(&self) -> bool {
        self.scale == 1.0
    }

    /// Bottom status bar band of a capture, in 1:1 game pixels.
    pub fn band(&self, img: &RgbaImage) -> RgbaImage {
        self.render(img, 0, u32::MAX)
    }

    // Band columns `x0..x0 + w` (clamped).
    fn render(&self, img: &RgbaImage, x0: u32, w: u32) -> RgbaImage {
        let (iw, ih) = img.dimensions();
        if self.is_native() {
            let y = ih.saturating_sub(BAND);
            let x0 = x0.min(iw);
            return imageops::crop_imm(img, x0, y, w.min(iw - x0), ih - y).to_image();
        }
        let s = self.scale;
        let full_w = ((iw as f32 - self.dx) / s).floor().max(0.0) as u32;
        let x0 = x0.min(full_w);
        let out_w = w.min(full_w - x0);
        let out_h = BAND.min((ih as f32 / s).floor() as u32);
        let top = ih as f32 - out_h as f32 * s - self.dy;
        // Sample each game pixel at its center inside the stretched image.
        RgbaImage::from_fn(out_w, out_h, |x, y| {
            let fx = self.dx + ((x0 + x) as f32 + 0.5) * s - 0.5;
            let fy = top + (y as f32 + 0.5) * s - 0.5;
            levels(bilinear(img, fx, fy))
        })
    }
}

/// Status bar layout for an already known view, or None when the labels do not match there.
pub fn locate_in(img: &RgbaImage, labels: &Labels, view: View) -> Option<Layout> {
    let band = view.band(img);
    if view.is_native() {
        locate::locate(&band, labels)
    } else {
        locate::locate_scored(&band, labels, LOOSE_MISMATCH).map(|(l, _)| l)
    }
}

/// Finds the capture's scale and status bar (1:1 first; scaled search is slow, so keep the view).
pub fn detect(img: &RgbaImage, fonts: &Fonts) -> Option<(View, Layout)> {
    if let Some(layout) = locate_in(img, &fonts.labels, View::NATIVE) {
        return Some((View::NATIVE, layout));
    }
    // Coarse: best scale by label match on the full band.
    let mut coarse: Option<(f64, View, Layout)> = None;
    for scale in SCALES {
        for view in phases(scale, COARSE_PHASES) {
            if let Some((layout, score)) = locate::locate_scored(&view.band(img), &fonts.labels, LOOSE_MISMATCH)
                && coarse.as_ref().is_none_or(|(b, _, _)| score < *b)
            {
                coarse = Some((score, view, layout));
            }
        }
    }
    let (_, view, layout) = coarse?;
    // Fine: best sub-pixel offset by how much of the status bar actually reads, on a small crop.
    let x0 = layout.level_label.x.min(layout.exp_label.x).saturating_sub(FINE_MARGIN);
    let x1 = (layout.level.x + layout.level.w).max(layout.exp.x + layout.exp.w) + FINE_MARGIN;
    let mut best: Option<((u32, f64), View)> = None;
    for v in phases(view.scale, FINE_PHASES) {
        let crop = v.render(img, x0, x1 - x0);
        let Some((l, score)) = locate::locate_scored(&crop, &fonts.labels, LOOSE_MISMATCH) else {
            continue;
        };
        let key = (misread(&crop, &l, fonts), score);
        if best.as_ref().is_none_or(|(b, _)| key < *b) {
            best = Some((key, v));
        }
    }
    let (_, view) = best?;
    Some((view, locate_in(img, &fonts.labels, view)?))
}

// Offsets on an n×n grid inside one stretched pixel group.
fn phases(scale: f32, n: u32) -> impl Iterator<Item = View> {
    let step = scale / n as f32;
    (0..n * n).map(move |i| View { scale, dx: (i % n) as f32 * step, dy: (i / n) as f32 * step })
}

// Penalty for a read: unknown glyphs, plus a large one per field that does not parse.
fn misread(img: &RgbaImage, layout: &Layout, fonts: &Fonts) -> u32 {
    let (exp_line, exp) = crate::read_exp(img, layout, fonts);
    let (level_line, level) = crate::read_level(img, layout, fonts);
    let unknown = (exp_line.unknown.len() + level_line.unknown.len()) as u32;
    unknown + 100 * u32::from(exp.is_none()) + 100 * u32::from(level.is_none())
}

fn bilinear(img: &RgbaImage, fx: f32, fy: f32) -> [f32; 3] {
    let (w, h) = img.dimensions();
    let fx = fx.clamp(0.0, (w - 1) as f32);
    let fy = fy.clamp(0.0, (h - 1) as f32);
    let (x0, y0) = (fx as u32, fy as u32);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (ax, ay) = (fx - x0 as f32, fy - y0 as f32);
    let [a, b, c, d] = [(x0, y0), (x1, y0), (x0, y1), (x1, y1)].map(|(x, y)| img.get_pixel(x, y).0);
    let mut out = [0.0; 3];
    for (i, o) in out.iter_mut().enumerate() {
        let top = f32::from(a[i]) * (1.0 - ax) + f32::from(b[i]) * ax;
        let bottom = f32::from(c[i]) * (1.0 - ax) + f32::from(d[i]) * ax;
        *o = top * (1.0 - ay) + bottom * ay;
    }
    out
}

fn levels(rgb: [f32; 3]) -> Rgba<u8> {
    let [r, g, b] = rgb.map(|v| ((v - LEVEL_LO) * 255.0 / (LEVEL_HI - LEVEL_LO)).clamp(0.0, 255.0) as u8);
    Rgba([r, g, b, 255])
}
