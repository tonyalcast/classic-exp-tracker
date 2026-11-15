//! Passive, read-only capture of the game window's pixels.

pub mod permission;

use std::fmt;

pub use image;
pub use image::RgbaImage;
use image::imageops;

/// Substring that identifies the game client by app name or title.
const GAME_MATCH: &str = "maplestory";
/// Substrings of windows that match the game name but are not the game.
const EXCLUDE: [&str; 1] = ["launcher"];

#[derive(Debug)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<xcap::XCapError> for Error {
    fn from(e: xcap::XCapError) -> Self {
        Self(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Rectangle in window pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Window metadata, used for listing and picking the game window.
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub id: u32,
    pub pid: u32,
    pub app_name: String,
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub minimized: bool,
}

impl WindowInfo {
    fn of(w: &xcap::Window) -> Option<Self> {
        Some(Self {
            id: w.id().ok()?,
            pid: w.pid().ok()?,
            app_name: w.app_name().unwrap_or_default(),
            title: w.title().unwrap_or_default(),
            x: w.x().unwrap_or_default(),
            y: w.y().unwrap_or_default(),
            width: w.width().unwrap_or_default(),
            height: w.height().unwrap_or_default(),
            minimized: w.is_minimized().unwrap_or(false),
        })
    }

    pub fn is_game(&self) -> bool {
        let hay = format!("{} {}", self.app_name, self.title).to_lowercase();
        hay.contains(GAME_MATCH) && !EXCLUDE.iter().any(|x| hay.contains(x))
    }
}

/// Lists all top-level windows visible to the OS capture API.
pub fn list_windows() -> Result<Vec<WindowInfo>> {
    Ok(xcap::Window::all()?.iter().filter_map(WindowInfo::of).collect())
}

/// Handle to the game window.
pub struct GameWindow {
    win: xcap::Window,
    pub info: WindowInfo,
}

impl GameWindow {
    /// Finds the game window, preferring the largest non-minimized match.
    pub fn find() -> Result<Option<Self>> {
        let best = xcap::Window::all()?
            .into_iter()
            .filter_map(|w| WindowInfo::of(&w).map(|i| (w, i)))
            .filter(|(_, i)| i.is_game() && !i.minimized && i.width > 0 && i.height > 0)
            .max_by_key(|(_, i)| u64::from(i.width) * u64::from(i.height));
        Ok(best.map(|(win, info)| Self { win, info }))
    }

    /// Captures the whole window content.
    pub fn capture(&self) -> Result<RgbaImage> {
        Ok(self.win.capture_image()?)
    }

    /// Captures the window and crops it to `r`, clamped to the image bounds.
    pub fn capture_region(&self, r: Rect) -> Result<RgbaImage> {
        Ok(crop(&self.capture()?, r))
    }
}

/// Crops `img` to `r`, clamped to the image bounds.
pub fn crop(img: &RgbaImage, r: Rect) -> RgbaImage {
    let x = r.x.min(img.width());
    let y = r.y.min(img.height());
    let w = r.w.min(img.width() - x);
    let h = r.h.min(img.height() - y);
    imageops::crop_imm(img, x, y, w, h).to_image()
}

/// FNV-1a hash of the pixels, used to skip OCR when nothing changed.
pub fn frame_hash(img: &RgbaImage) -> u64 {
    img.as_raw().iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_clamps_to_bounds() {
        let img = RgbaImage::new(10, 10);
        let c = crop(&img, Rect { x: 8, y: 8, w: 5, h: 5 });
        assert_eq!((c.width(), c.height()), (2, 2));
    }

    #[test]
    fn hash_changes_with_pixels() {
        let mut img = RgbaImage::new(4, 4);
        let a = frame_hash(&img);
        img.put_pixel(1, 1, image::Rgba([1, 2, 3, 255]));
        assert_ne!(a, frame_hash(&img));
    }

    #[test]
    fn game_matching_skips_launcher() {
        let mk = |app: &str, title: &str| WindowInfo {
            id: 1,
            pid: 1,
            app_name: app.into(),
            title: title.into(),
            x: 0,
            y: 0,
            width: 800,
            height: 600,
            minimized: false,
        };
        assert!(mk("MapleStory Classic", "").is_game());
        assert!(mk("wine", "MapleStory").is_game());
        assert!(!mk("MapleStory Launcher", "").is_game());
        assert!(!mk("Safari", "Maple syrup").is_game());
    }
}
