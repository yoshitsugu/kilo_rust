#[macro_use]
extern crate bitflags;

use std::io;

mod file_syntax;
mod highlight;
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

    window.editor_set_status_mssage("HELP: Ctrl-X Ctrl-S = save | Ctrl-Q = quit | Ctrl-S = search");

    loop {
        window.refresh_screen()?;
        match raw.process_keypress(&mut window)? {
            LoopStatus::CONTINUE => {}
            LoopStatus::STOP => break,
        }
    }
    Ok(())
}
