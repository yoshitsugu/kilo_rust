use std::io::{self, Read, Write};

pub struct Window {
    pub height: u16,
    pub width: u16,
    pub stdout: io::Stdout,
}

impl Window {
    pub fn new(mut stdin: &mut io::Stdin) -> Result<Window, io::Error> {
        let mut stdout = io::stdout();
        match get_window_size(&mut stdin, &mut stdout) {
            Ok(Some((width, height))) => Ok(Window {
                width,
                height,
                stdout,
            }),
            Ok(_) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid window size",
            )),
            Err(e) => Err(e),
        }
    }

    pub fn refresh_screen(&mut self) -> io::Result<()> {
        write!(self.stdout, "\x1b[2J")?;
        write!(self.stdout, "\x1b[H")?;
        self.editor_draw_rows()?;
        write!(self.stdout, "\x1b[H")?;
        self.stdout.flush()?;
        Ok(())
    }

    fn editor_draw_rows(&mut self) -> io::Result<()> {
        for _ in 0..self.height {
            write!(self.stdout, "~\r\n")?;
        }
        self.stdout.flush()?;
        Ok(())
    }
}

fn get_cursor_position(stdin: &mut io::Stdin) -> io::Result<Option<(u16, u16)>> {
    let mut bytes: Vec<u8> = vec![];
    for (i, b) in stdin.bytes().enumerate() {
        bytes.push(b.unwrap_or(0));
        if bytes[i] == b'R' || i > 31 {
            println!("bytes:{}, {}", bytes[i], b'R');
            break;
        }
    }
    if bytes[0] != b'\x1b' || bytes[1] != b'[' {
        return Ok(None);
    }
    let byte_chars = String::from_utf8(bytes[2..bytes.len() - 1].to_vec()).unwrap();
    let splitted: Vec<&str> = byte_chars.split(";").collect();
    if splitted.len() >= 2 {
        return Ok(Some((
            splitted[1].parse::<u16>().unwrap(),
            splitted[0].parse::<u16>().unwrap(),
        )));
    }
    Ok(None)
}

fn get_window_size(
    stdin: &mut io::Stdin,
    stdout: &mut io::Stdout,
) -> io::Result<Option<(u16, u16)>> {
    use libc::{ioctl, winsize, STDOUT_FILENO, TIOCGWINSZ};
    use std::{fs::File, mem, os::unix::io::IntoRawFd};

    let fd = if let Ok(file) = File::open("/dev/tty") {
        file.into_raw_fd()
    } else {
        STDOUT_FILENO
    };

    let mut ws: winsize = unsafe { mem::zeroed() };
    if unsafe { ioctl(fd, TIOCGWINSZ, &mut ws) } == -1 {
        write!(stdout, "\x1b[9999C\x1b[9999B\x1b[6n")?;
        stdout.flush()?;
        get_cursor_position(stdin)
    } else {
        Ok(Some((ws.ws_col, ws.ws_row)))
    }
}
