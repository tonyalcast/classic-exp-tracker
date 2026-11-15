// Runs the OS OCR on the pure-white pixels of a region: `sysbits <png> <x> <y> <w> <h>`.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    let n = |i: usize| a[i].parse::<u32>();
    let img = image::open(&a[1])?.to_rgba8();
    let (x0, y0, w, h) = (n(2)?, n(3)?, n(4)?, n(5)?);
    let ink: Vec<bool> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| img.get_pixel(x0 + x, y0 + y).0[..3].iter().all(|&c| c > 200))
        .collect();
    println!("{:?}", sysocr::read_bits(w, h, &ink)?);
    Ok(())
}
