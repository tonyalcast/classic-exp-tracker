// Crops and upscales a PNG for inspection: `crop <in> <out> <x> <y> <w> <h> [scale]`.
use capture::image::{self, imageops::FilterType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    let n = |i: usize| a[i].parse::<u32>();
    let img = image::open(&a[1])?.to_rgba8();
    let r = capture::Rect { x: n(3)?, y: n(4)?, w: n(5)?, h: n(6)? };
    let c = capture::crop(&img, r);
    let s = a.get(7).map(|v| v.parse::<u32>()).transpose()?.unwrap_or(1);
    image::imageops::resize(&c, c.width() * s, c.height() * s, FilterType::Nearest).save(&a[2])?;
    Ok(())
}
