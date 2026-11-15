// Prints a region as luminance/colour codes for glyph analysis: `pixels <in> <x> <y> <w> <h>`.
use capture::image;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    let n = |i: usize| a[i].parse::<u32>().unwrap();
    let img = image::open(&a[1])?.to_rgba8();
    let (x0, y0, w, h) = (n(2), n(3), n(4), n(5));
    for y in y0..y0 + h {
        let mut line = format!("{y:>5} ");
        for x in x0..x0 + w {
            let p = img.get_pixel(x, y).0;
            let (r, g, b) = (p[0] as u32, p[1] as u32, p[2] as u32);
            let lum = (299 * r + 587 * g + 114 * b) / 1000;
            let c = if r.min(g).min(b) > 200 { '#' } else if g > 150 && r < 180 && b < 100 { 'g' } else if r > 200 && g < 170 && b < 80 { 'o' } else if lum > 120 { '+' } else if lum > 80 { ':' } else { '.' };
            line.push(c);
        }
        println!("{line}");
    }
    Ok(())
}
