use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::Instant;

use base64::Engine;
use capture::image::{self, ImageFormat, imageops::FilterType};
use capture::{GameWindow, WindowInfo};
use serde::Serialize;

const PREVIEW_WIDTH: u32 = 480;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowDto {
    pub id: u32,
    pub pid: u32,
    pub app_name: String,
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl From<&WindowInfo> for WindowDto {
    fn from(w: &WindowInfo) -> Self {
        Self {
            id: w.id,
            pid: w.pid,
            app_name: w.app_name.clone(),
            title: w.title.clone(),
            x: w.x,
            y: w.y,
            width: w.width,
            height: w.height,
        }
    }
}

/// Result of one "detect game window" check.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detection {
    pub permission: bool,
    pub visible_windows: usize,
    pub window: Option<WindowDto>,
    pub capture_ms: Option<f64>,
    pub image_width: Option<u32>,
    pub image_height: Option<u32>,
    /// PNG data URL of a downscaled capture.
    pub preview: Option<String>,
    pub saved_path: Option<String>,
    /// Where the status bar was found, e.g. "1:1" or "125% scaling"; None when not found.
    pub status_bar: Option<String>,
}

/// Finds the game window, captures it once and saves the full frame to `out_dir`.
pub fn detect(out_dir: &Path) -> Result<Detection, String> {
    let permission = capture::permission::granted();
    let visible_windows = capture::list_windows().map_err(|e| e.to_string())?.len();
    let mut d = Detection {
        permission,
        visible_windows,
        window: None,
        capture_ms: None,
        image_width: None,
        image_height: None,
        preview: None,
        saved_path: None,
        status_bar: None,
    };
    let Some(game) = GameWindow::find().map_err(|e| e.to_string())? else {
        return Ok(d);
    };
    d.window = Some((&game.info).into());
    let t = Instant::now();
    let img = game.capture().map_err(|e| e.to_string())?;
    d.capture_ms = Some(t.elapsed().as_secs_f64() * 1000.0);
    d.image_width = Some(img.width());
    d.image_height = Some(img.height());
    d.saved_path = Some(save(&img, out_dir)?.display().to_string());
    d.preview = Some(preview(&img)?);
    d.status_bar = ocr::scale::detect(&img, &ocr::Fonts::builtin()).map(|(view, _)| {
        if view.is_native() { "1:1".into() } else { format!("{:.0}% scaling", view.scale * 100.0) }
    });
    Ok(d)
}

fn save(img: &capture::RgbaImage, dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = dir.join("last-capture.png");
    img.save(&path).map_err(|e| e.to_string())?;
    Ok(path)
}

fn preview(img: &capture::RgbaImage) -> Result<String, String> {
    let h = (img.height() as f64 * PREVIEW_WIDTH as f64 / img.width().max(1) as f64).round() as u32;
    let small = image::imageops::resize(img, PREVIEW_WIDTH, h.max(1), FilterType::Triangle);
    let mut buf = Cursor::new(Vec::new());
    small.write_to(&mut buf, ImageFormat::Png).map_err(|e| e.to_string())?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
    Ok(format!("data:image/png;base64,{b64}"))
}
