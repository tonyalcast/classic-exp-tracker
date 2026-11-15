// Lists candidate windows and saves one capture of the game window: `cargo run -p capture --example snap -- out.png`.

use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args().nth(1).unwrap_or_else(|| "game.png".into());
    for w in capture::list_windows()? {
        if w.is_game() || w.app_name.to_lowercase().contains("wine") {
            println!("candidate: {w:?}");
        }
    }
    let Some(game) = capture::GameWindow::find()? else {
        return Err("game window not found".into());
    };
    println!("game: {:?}", game.info);
    let t = Instant::now();
    let img = game.capture()?;
    println!("captured {}x{} in {:?}", img.width(), img.height(), t.elapsed());
    img.save(&out)?;
    println!("saved {out}");
    Ok(())
}
