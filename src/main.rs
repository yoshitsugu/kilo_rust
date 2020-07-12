use std::io;

mod input;
mod window;
use crate::input::*;
use crate::window::*;

fn main() -> io::Result<()> {
    let mut raw = RawMode::new()?;
    let window = Window::new()?;
    window.refresh_screen()?;
    raw.process_keypress()?;
    Ok(())
}
