use crate::input::CursorMoveDirection;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::time::{Duration, Instant};

pub struct Window {
    pub cx: usize, // 文字列上でのカーソル位置
    pub rx: usize, // 実際にレンダリングされたカーソル位置
    pub cy: usize,
    pub rows: usize,
    pub columns: usize,
    pub text_rows: usize,
    pub row_offset: usize,
    pub col_offset: usize,
    pub stdout: io::Stdout,
    pub text_buffer: String,
    pub content_buffer: Vec<String>,
    pub render_buffer: Vec<String>,
    pub filename: Option<String>,
    pub status_message: String,
    pub message_time: Instant,
}

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const KILO_TAB_STOP: usize = 8;
const DISPLAY_STATUS_MESSAGE_DURATION: u64 = 5;

impl Window {
    pub fn new(mut stdin: &mut io::Stdin) -> Result<Window, io::Error> {
        let mut stdout = io::stdout();
        match get_window_size(&mut stdin, &mut stdout) {
            Ok(Some((columns, rows))) => Ok(Window {
                cx: 0,
                rx: 0,
                cy: 0,
                columns: columns as usize,
                rows: (rows as usize) - 2,
                text_rows: 0,
                row_offset: 0,
                col_offset: 0,
                stdout,
                text_buffer: String::new(),
                content_buffer: vec![],
                render_buffer: vec![],
                filename: None,
                status_message: String::new(),
                message_time: Instant::now(),
            }),
            Ok(_) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid window size",
            )),
            Err(e) => Err(e),
        }
    }

    fn editor_draw_status_bar(&mut self) {
        let status_left = format!(
            "{}",
            (self.filename.clone()).unwrap_or("[NO NAME]".to_string()),
        );
        let status_right = format!("{}/{}", self.cy, self.content_buffer.len());
        self.text_buffer.push_str(&format!(
            "\x1b[7m{}{}{}\x1b[m\r\n",
            status_left,
            (0..(self.columns - (status_left.len() + status_right.len())))
                .map(|_| " ")
                .collect::<String>(),
            status_right
        ));
    }

    fn editor_draw_message_bar(&mut self) {
        self.text_buffer.push_str("\x1b[K");
        if Instant::now() - self.message_time < Duration::from_secs(DISPLAY_STATUS_MESSAGE_DURATION)
        {
            self.text_buffer.push_str(&self.status_message);
        }
    }

    pub fn editor_set_status_mssage<T: ToString>(&mut self, message: T) {
        self.status_message = message.to_string();
        self.message_time = Instant::now();
    }

    pub fn insert_char(&mut self, c: char) {
        use std::cmp::min;
        if self.cy == self.content_buffer.len() {
            self.content_buffer.push(String::new());
            self.render_buffer.push(String::new());
        }
        let at = min(self.cx, self.content_buffer[self.cy].len());
        self.content_buffer[self.cy].insert(at, c);
        self.render_buffer[self.cy] = self.to_render_line(&self.content_buffer[self.cy]);
        self.cx += 1;
    }

    pub fn refresh_screen(&mut self) -> io::Result<()> {
        self.editor_scroll();
        self.text_buffer.push_str("\x1b[?25l\x1b[H");
        self.editor_draw_rows()?;
        self.editor_draw_status_bar();
        self.editor_draw_message_bar();
        self.text_buffer.push_str(&format!(
            "\x1b[{};{}H",
            (self.cy - self.row_offset) + 1,
            (self.rx - self.col_offset) + 1
        ));
        self.text_buffer.push_str("\x1b[?25h");
        write!(self.stdout, "{}", self.text_buffer)?;
        self.stdout.flush()?;
        self.text_buffer.clear();
        Ok(())
    }

    fn editor_draw_rows(&mut self) -> io::Result<()> {
        use std::cmp::min;
        for y in 0..self.rows {
            let filerow = y + self.row_offset;
            if filerow >= self.text_rows {
                if self.text_rows == 0 && y == self.rows / 3 {
                    let welcome = format!("Kilo in Rust -- version {}", VERSION);
                    let mut padding = (self.columns - welcome.len()) / 2;
                    if padding > 0 {
                        self.text_buffer.push_str("~");
                        padding -= 1;
                    }
                    for _ in 0..padding {
                        self.text_buffer.push_str(" ");
                    }
                    self.text_buffer
                        .push_str(&welcome[..min(welcome.len(), self.columns)])
                } else {
                    self.text_buffer.push_str("~");
                }
            } else {
                if let Some(line) = &self.render_buffer.get(filerow) {
                    let line_min = if line.len() > 0 && self.col_offset < line.len() {
                        self.col_offset
                    } else {
                        0
                    };
                    let line_max = if self.col_offset < line.len() {
                        min(line.len(), self.columns + self.col_offset)
                    } else {
                        0
                    };
                    self.text_buffer.push_str(&line[line_min..line_max]);
                }
            }
            self.text_buffer.push_str("\x1b[K");
            self.text_buffer.push_str("\r\n");
        }
        Ok(())
    }

    pub fn move_cursor(&mut self, direction: CursorMoveDirection) {
        use std::cmp::min;
        use CursorMoveDirection::*;
        match direction {
            Down => {
                if self.text_rows > self.cy {
                    self.cy += 1;
                }
            }
            Up => {
                if 0 < self.cy {
                    self.cy -= 1;
                }
            }
            Right => {
                if let Some(line) = self.content_buffer.get(self.cy) {
                    if self.cx < line.len() {
                        self.cx += 1;
                    } else if self.cx == line.len() {
                        self.cy += 1;
                        self.cx = 0;
                    }
                }
            }
            Left => {
                if self.cx > 0 {
                    self.cx -= 1;
                } else if self.cy > 0 {
                    self.cy -= 1;
                    let line_length = match self.content_buffer.get(self.cy) {
                        Some(line) => line.len(),
                        _ => 0,
                    };
                    self.cx = line_length;
                }
            }
            PageUp => {
                self.cy = self.row_offset;
                for _ in 0..self.rows {
                    self.move_cursor(Up);
                }
            }
            PageDown => {
                self.cy = self.row_offset + self.rows - 1;
                if self.cy > self.content_buffer.len() {
                    self.cy = self.content_buffer.len();
                }
                for _ in 0..self.rows {
                    self.move_cursor(Down);
                }
            }
            LineTop => self.cx = 0,
            LineBottom => {
                if let Some(line) = self.content_buffer.get(self.cy) {
                    self.cx = min(self.columns + self.col_offset - 1, line.len());
                } else {
                    self.cx = 0;
                }
            }
        };
        let line_length = match self.content_buffer.get(self.cy) {
            Some(line) => line.len(),
            _ => 0,
        };
        self.cx = min(self.cx, line_length);
    }

    fn cx_to_rx(&self, line: &String) -> usize {
        let mut rx = 0;
        for (char_index, char) in line.chars().enumerate() {
            if self.cx == char_index {
                break;
            }
            if char == '\t' {
                rx += (KILO_TAB_STOP - 1) - (rx % KILO_TAB_STOP);
            }
            rx += 1
        }
        rx
    }

    pub fn editor_scroll(&mut self) {
        self.rx = 0;
        if self.cy < self.content_buffer.len() {
            self.rx = self.cx_to_rx(&self.content_buffer[self.cy]);
        }
        if self.cy < self.row_offset {
            self.row_offset = self.cy;
        }
        if self.cy >= self.row_offset + self.rows {
            self.row_offset = self.cy - self.rows + 1;
        }
        if self.rx < self.col_offset {
            self.col_offset = self.rx
        }
        if self.rx >= self.col_offset + self.columns {
            self.col_offset = self.rx - self.columns + 1
        }
    }

    pub fn open_file(&mut self, filename: String) -> io::Result<()> {
        self.filename = Some(filename.to_string());
        for line in BufReader::new(File::open(filename)?).lines() {
            let line = line?;
            self.render_buffer.push(self.to_render_line(&line));
            self.content_buffer.push(line);
            self.text_rows += 1;
        }
        Ok(())
    }

    fn to_render_line(&self, line: &String) -> String {
        let mut string = String::new();
        for (char_index, char) in line.chars().enumerate() {
            if char == '\t' {
                string.push(' ');
                let mut m = char_index + 1;
                while m % KILO_TAB_STOP != 0 {
                    string.push(' ');
                    m += 1;
                }
            } else {
                string.push(char);
            }
        }
        string
    }
}

fn get_cursor_position(stdin: &mut io::Stdin) -> io::Result<Option<(u16, u16)>> {
    let mut bytes: Vec<u8> = vec![];
    for b in stdin.bytes() {
        bytes.push(b.unwrap_or(0));
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
    use std::{mem, os::unix::io::IntoRawFd};

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
