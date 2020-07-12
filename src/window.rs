use std::io::{self, Write};

pub struct Window {
    pub height: u16,
    pub width: u16,
}

impl Window {
    pub fn new() -> Result<Window, io::Error> {
        match get_window_size() {
            Ok(Some((width, height))) => Ok(Window { width, height }),
            Ok(_) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid window size",
            )),
            Err(e) => Err(e),
        }
    }

    pub fn refresh_screen(&self) -> io::Result<()> {
        write!(io::stdout(), "\x1b[2J")?;
        write!(io::stdout(), "\x1b[H")?;
        self.editor_draw_rows()?;
        write!(io::stdout(), "\x1b[H")?;
        io::stdout().flush()?;
        Ok(())
    }

    fn editor_draw_rows(&self) -> io::Result<()> {
        for _ in 0..self.height {
            write!(io::stdout(), "~\r\n")?;
        }
        io::stdout().flush()?;
        Ok(())
    }
}

fn get_window_size() -> io::Result<Option<(u16, u16)>> {
    use libc::{ioctl, winsize, STDOUT_FILENO, TIOCGWINSZ};
    use std::{fs::File, mem, os::unix::io::IntoRawFd};

    let fd = if let Ok(file) = File::open("/dev/tty") {
        file.into_raw_fd()
    } else {
        STDOUT_FILENO
    };

    let mut ws: winsize = unsafe { mem::zeroed() };
    if unsafe { ioctl(fd, TIOCGWINSZ, &mut ws) } == -1 {
        Ok(None)
    } else {
        Ok(Some((ws.ws_col, ws.ws_row)))
    }
}
