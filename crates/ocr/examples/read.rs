// Locates the status bar in a full window capture and prints what it reads: `read <png>`.
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage: read <png>")?;
    let img = image::open(path)?.to_rgba8();
    let fonts = ocr::Fonts::builtin();
    let t = Instant::now();
    let layout = ocr::locate::locate(&img, &fonts.labels).ok_or("status bar not found")?;
    println!("locate: {:?} in {:?}", layout, t.elapsed());
    let t = Instant::now();
    let f = ocr::read_frame(&img, &layout, &fonts);
    println!("read: exp={:?} level={:?} text={:?}/{:?} in {:?}", f.exp, f.level, f.exp_line.text, f.level_line.text, t.elapsed());
    Ok(())
}
