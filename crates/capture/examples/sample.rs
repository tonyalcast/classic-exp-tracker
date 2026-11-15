// Prints hex colors at given points: `sample <png> <x,y> [<x,y> ...]`.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    let img = capture::image::open(&a[1])?.to_rgba8();
    for p in &a[2..] {
        let (x, y) = p.split_once(',').ok_or("x,y")?;
        let c = img.get_pixel(x.parse()?, y.parse()?).0;
        println!("{p}: #{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
    }
    Ok(())
}
