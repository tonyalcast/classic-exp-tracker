// Adds glyph templates from a labeled capture: `calibrate <png> <status|level|label> <x> <y> <w> <h> <text> <out>`.
use ocr::{GlyphSet, Ink, Mask, Rect, glyph};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    let img = image::open(&a[1])?.to_rgba8();
    let n = |i: usize| a[i].parse::<u32>();
    let rect = Rect::new(n(3)?, n(4)?, n(5)?, n(6)?);
    let (text, out) = (&a[7], &a[8]);
    let mut set = std::fs::read_to_string(out).ok().map(|t| GlyphSet::from_text(&t)).transpose()?.unwrap_or_default();
    let ink = match a[2].as_str() {
        "status" => Ink::Status,
        _ => Ink::White,
    };
    let mask = Mask::from_image(&img, rect, ink);
    let glyphs = if a[2] == "label" {
        // A label is stored whole, trimmed to its ink bounds.
        let x0 = (0..mask.w).find(|&x| mask.col_has_ink(x)).ok_or("no ink")?;
        let x1 = (0..mask.w).rev().find(|&x| mask.col_has_ink(x)).unwrap();
        let y0 = (0..mask.h).find(|&y| mask.row_has_ink(y)).unwrap();
        let y1 = (0..mask.h).rev().find(|&y| mask.row_has_ink(y)).unwrap();
        vec![glyph::Glyph { x: 0, top: 0, mask: mask.sub(Rect::new(x0, y0, x1 - x0 + 1, y1 - y0 + 1)) }]
    } else {
        glyph::segment(&mask)
    };
    let chars: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    if chars.len() != glyphs.len() {
        for g in &glyphs {
            println!("x={} top={}\n{}\n", g.x, g.top, g.mask.to_rows().join("\n"));
        }
        return Err(format!("{} glyphs found, {} chars given", glyphs.len(), chars.len()).into());
    }
    let added = chars.iter().zip(&glyphs).filter(|(c, g)| set.add(**c, g)).count();
    std::fs::write(out, set.to_text())?;
    println!("added {added}, total {} ({:?})", set.len(), set.chars());
    Ok(())
}
