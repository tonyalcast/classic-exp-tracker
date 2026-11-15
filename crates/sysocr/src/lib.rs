//! One-shot text recognition through the OS engine (Apple Vision, Windows.Media.Ocr).
//!
//! Used only for rare reads with an open alphabet, such as the character name.

use image::RgbaImage;
use image::imageops::{self, FilterType};

/// Upscale factor that makes tiny pixel fonts readable for the OS engine.
const SCALE: u32 = 4;
/// Blank margin around the text, in source pixels.
const MARGIN: u32 = 6;

/// Recognizes the text in `img`; lines are joined with a space.
pub fn read_text(img: &RgbaImage) -> Result<String, String> {
    let prepared = prepare(img);
    let lines = imp::recognize(&prepared)?;
    Ok(lines.join(" ").trim().to_string())
}

/// Recognizes a binary bitmap rendered as black ink on white; `ink[y * w + x]` is true for ink.
pub fn read_bits(w: u32, h: u32, ink: &[bool]) -> Result<String, String> {
    let img = RgbaImage::from_fn(w, h, |x, y| {
        if ink[(y * w + x) as usize] { image::Rgba([0, 0, 0, 255]) } else { image::Rgba([255, 255, 255, 255]) }
    });
    read_text(&img)
}

// Pads with the edge color and upscales without smoothing.
fn prepare(img: &RgbaImage) -> RgbaImage {
    let bg = *img.get_pixel(0, 0);
    let mut padded = RgbaImage::from_pixel(img.width() + 2 * MARGIN, img.height() + 2 * MARGIN, bg);
    imageops::replace(&mut padded, img, MARGIN as i64, MARGIN as i64);
    imageops::resize(&padded, padded.width() * SCALE, padded.height() * SCALE, FilterType::Nearest)
}

#[cfg(target_os = "macos")]
mod imp {
    use std::io::Cursor;

    use image::{ImageFormat, RgbaImage};
    use objc2::AnyThread;
    use objc2::rc::{Retained, autoreleasepool};
    use objc2_foundation::{NSArray, NSData, NSDictionary};
    use objc2_vision::{VNImageRequestHandler, VNRecognizeTextRequest, VNRequest, VNRequestTextRecognitionLevel};

    pub fn recognize(img: &RgbaImage) -> Result<Vec<String>, String> {
        let mut png = Cursor::new(Vec::new());
        img.write_to(&mut png, ImageFormat::Png).map_err(|e| e.to_string())?;
        autoreleasepool(|_| {
            let data = NSData::with_bytes(png.get_ref());
            let handler =
                VNImageRequestHandler::initWithData_options(VNImageRequestHandler::alloc(), &data, &NSDictionary::new());
            let request = VNRecognizeTextRequest::new();
            request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
            request.setUsesLanguageCorrection(false);
            let base: Retained<VNRequest> = Retained::into_super(Retained::into_super(request.clone()));
            handler
                .performRequests_error(&NSArray::from_retained_slice(&[base]))
                .map_err(|e| e.localizedDescription().to_string())?;
            let lines = request
                .results()
                .map(|results| {
                    results
                        .iter()
                        .filter_map(|obs| obs.topCandidates(1).firstObject())
                        .map(|t| t.string().to_string())
                        .collect()
                })
                .unwrap_or_default();
            Ok(lines)
        })
    }
}

#[cfg(windows)]
mod imp {
    use image::RgbaImage;
    use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Security::Cryptography::CryptographicBuffer;

    pub fn recognize(img: &RgbaImage) -> Result<Vec<String>, String> {
        let e = |e: windows::core::Error| e.message().to_string();
        // Windows OCR expects BGRA.
        let mut bgra = img.as_raw().clone();
        for px in bgra.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
        let buffer = CryptographicBuffer::CreateFromByteArray(&bgra).map_err(e)?;
        let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
            &buffer,
            BitmapPixelFormat::Bgra8,
            img.width() as i32,
            img.height() as i32,
            BitmapAlphaMode::Premultiplied,
        )
        .map_err(e)?;
        let engine = OcrEngine::TryCreateFromUserProfileLanguages().map_err(e)?;
        let result = engine.RecognizeAsync(&bitmap).map_err(e)?.join().map_err(e)?;
        let mut lines = Vec::new();
        for line in result.Lines().map_err(e)? {
            lines.push(line.Text().map_err(e)?.to_string());
        }
        Ok(lines)
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
mod imp {
    use image::RgbaImage;

    pub fn recognize(_: &RgbaImage) -> Result<Vec<String>, String> {
        Err("OS OCR is not available on this platform".into())
    }
}
