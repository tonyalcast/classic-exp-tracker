// Runs the OS OCR on a region of a PNG: `sysread <png> <x> <y> <w> <h>`.
use image::imageops;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    let n = |i: usize| a[i].parse::<u32>();
    let img = image::open(&a[1])?.to_rgba8();
    let crop = imageops::crop_imm(&img, n(2)?, n(3)?, n(4)?, n(5)?).to_image();
    let t = std::time::Instant::now();
    println!("{:?} in {:?}", sysocr::read_text(&crop)?, t.elapsed());
    Ok(())
}
