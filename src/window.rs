use crate::input::CursorMoveDirection;
use std::io::{self, Read, Write};

pub struct Window {
    pub cx: usize,
    pub cy: usize,
    pub rows: u16,
    pub columns: u16,
    pub stdout: io::Stdout,
    pub text_buffer: String,
}

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

impl Window {
    pub fn new(mut stdin: &mut io::Stdin) -> Result<Window, io::Error> {
        let mut stdout = io::stdout();
        match get_window_size(&mut stdin, &mut stdout) {
            Ok(Some((columns, rows))) => Ok(Window {
                cx: 0,
                cy: 0,
                columns,
                rows,
                stdout,
                text_buffer: "".to_string(),
            }),
            Ok(_) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid window size",
            )),
            Err(e) => Err(e),
        }
    }

    pub fn refresh_screen(&mut self) -> io::Result<()> {
        self.text_buffer.push_str("\x1b[?25l\x1b[H");
        self.editor_draw_rows()?;
        self.text_buffer
            .push_str(&format!("\x1b[{};{}H", self.cy + 1, self.cx + 1));
        self.text_buffer.push_str("\x1b[?25h");

        write!(self.stdout, "{}", self.text_buffer)?;
        self.stdout.flush()?;
        self.text_buffer.clear();
        Ok(())
    }

    fn editor_draw_rows(&mut self) -> io::Result<()> {
        use std::cmp::min;
        for y in 0..self.rows {
            if y == self.rows / 3 {
                let welcome = format!("Kilo in Rust -- version {}", VERSION);
                let mut padding = (self.columns as usize - welcome.len()) / 2;
                if padding > 0 {
                    self.text_buffer.push_str("~");
                    padding -= 1;
                }
                for _ in 0..padding {
                    self.text_buffer.push_str(" ");
                }
                self.text_buffer
                    .push_str(&welcome[..min(welcome.len(), self.columns as usize)])
            } else {
                self.text_buffer.push_str("~");
            }
            self.text_buffer.push_str("\x1b[K");
            if y < self.rows - 1 {
                self.text_buffer.push_str("\r\n");
            }
        }
        Ok(())
    }

    pub fn move_cursor(&mut self, direction: CursorMoveDirection) {
        use CursorMoveDirection::*;
        match direction {
            Down => {
                if self.rows as usize > self.cy {
                    self.cy += 1
                }
            }
            Up => {
                if 0 < self.cy {
                    self.cy -= 1
                }
            }
            Right => {
                if self.columns as usize > self.cx {
                    self.cx += 1;
                }
            }
            Left => {
                if self.cx > 0 {
                    self.cx -= 1;
                }
            }
            Top => self.cy = 0,
            Bottom => self.cy = (self.rows - 1) as usize,
            LineTop => self.cx = 0,
            LineBottom => self.cx = (self.columns - 1) as usize,
        }
    }
}

fn get_cursor_position(stdin: &mut io::Stdin) -> io::Result<Option<(u16, u16)>> {
    let mut bytes: Vec<u8> = vec![];
    for (i, b) in stdin.bytes().enumerate() {
        bytes.push(b.unwrap_or(0));
        if bytes[i] == b'R' {
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
