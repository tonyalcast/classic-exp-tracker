// Prints every window the capture API can see.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("screen permission: {}", capture::permission::granted());
    for w in capture::list_windows()? {
        println!("{:>6} pid={:<6} {:>4}x{:<4} app={:?} title={:?}", w.id, w.pid, w.width, w.height, w.app_name, w.title);
    }
    Ok(())
}
