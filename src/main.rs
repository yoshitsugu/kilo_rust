use std::io;

mod input;
mod window;
use crate::input::*;
use crate::window::*;

fn main() -> io::Result<()> {
    let mut raw = RawMode::new()?;
    let mut window = Window::new(&mut raw.stdin)?;
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 {
        window.open_file(args[1].to_string())?;
    }

    loop {
        window.refresh_screen()?;
        match raw.process_keypress(&mut window)? {
            LoopStatus::CONTINUE => {}
            LoopStatus::STOP => break,
        }
    }
    Ok(())
}
