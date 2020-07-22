use crate::input::{CursorMoveDirection, LoopStatus, RawMode};
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct Window {
    pub cx: usize, // 文字列上でのカーソル位置
    pub rx: usize, // 実際にレンダリングされたカーソル位置
    pub cy: usize,
    pub rows: usize,
    pub columns: usize,
    pub row_offset: usize,
    pub col_offset: usize,
    pub stdout: io::Stdout,
    pub text_buffer: String,
    pub content_buffer: Vec<String>,
    pub render_buffer: Vec<String>,
    pub filename: Option<PathBuf>,
    pub status_message: String,
    pub message_time: Instant,
    pub dirty: bool,
    pub quit_confirming: bool,
}

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const KILO_TAB_STOP: usize = 8;
const DISPLAY_STATUS_MESSAGE_DURATION: u64 = 3;

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
                row_offset: 0,
                col_offset: 0,
                stdout,
                text_buffer: String::new(),
                content_buffer: vec![],
                render_buffer: vec![],
                filename: None,
                status_message: String::new(),
                message_time: Instant::now(),
                dirty: false,
                quit_confirming: false,
            }),
            Ok(_) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid window size",
            )),
            Err(e) => Err(e),
        }
    }

    fn editor_draw_status_bar(&mut self) {
        let filename = if let Some(path) = &self.filename {
            match path.file_name() {
                Some(osstr) => osstr.to_str().unwrap_or("[NO NAME]").to_string(),
                None => "[NO NAME]".to_string(),
            }
        } else {
            "[NO NAME]".to_string()
        };
        let dirty_symbol = if self.dirty { "*" } else { "" };
        let status_left = format!("{}{}", filename, dirty_symbol);
        let status_right = format!("{}/{}", self.cy + 1, self.content_buffer.len());
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
        self.dirty = true;
    }

    pub fn delete_char(&mut self) {
        if self.cy == self.rows {
            return;
        }
        if self.cx == 0 && self.cy == 0 {
            return;
        }
        if self.cx > 0 {
            self.content_buffer[self.cy].remove(self.cx - 1);
            self.cx -= 1;
            self.render_buffer[self.cy] = self.to_render_line(&self.content_buffer[self.cy]);
        } else {
            self.cx = self.content_buffer[self.cy - 1].len();
            let line = &self.content_buffer[self.cy].clone();
            self.content_buffer[self.cy - 1].push_str(&line);
            self.render_buffer[self.cy - 1] =
                self.to_render_line(&self.content_buffer[self.cy - 1]);
            self.content_buffer.remove(self.cy);
            self.render_buffer.remove(self.cy);
            self.cy -= 1;
        }
        self.dirty = true;
    }

    pub fn break_line(&mut self) {
        let line = &self.content_buffer[self.cy].clone();
        let remain = &line[..self.cx];
        let rest = &line[self.cx..line.len()];
        self.content_buffer[self.cy] = remain.to_string();
        self.content_buffer.insert(self.cy + 1, rest.to_string());
        self.render_buffer[self.cy] = self.to_render_line(&remain.to_string());
        self.render_buffer
            .insert(self.cy + 1, self.to_render_line(&rest.to_string()));
        self.cy += 1;
        self.cx = 0;
        self.dirty = true;
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
            if self.filename.is_none() && filerow >= self.content_buffer.len() {
                if self.content_buffer.len() == 0 && y == self.rows / 3 {
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
                } else {
                    self.text_buffer.push_str("~");
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
                if self.content_buffer.len() > self.cy {
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

    fn rx_to_cx(&self, rx: usize, line: &String) -> usize {
        let mut cur_rx = 0;
        for (cx, rc) in line.chars().enumerate() {
            if rc == '\t' {
                cur_rx += (KILO_TAB_STOP - 1) - (cur_rx % KILO_TAB_STOP);
            }
            cur_rx += 1;
            if cur_rx > rx {
                return cx;
            }
        }
        return line.len();
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
        use std::fs::canonicalize;
        use std::path::Path;
        self.filename = Some(canonicalize(Path::new(&filename))?);
        for line in BufReader::new(File::open(filename)?).lines() {
            let line = line?;
            self.render_buffer.push(self.to_render_line(&line));
            self.content_buffer.push(line);
        }
        Ok(())
    }

    fn editor_prompt(
        &mut self,
        input: &mut RawMode,
        format: &str,
        callback: Option<fn(&mut Self, &str, u8)>,
    ) -> io::Result<Option<String>> {
        use crate::input::InputType::*;
        let mut prompt_buffer = String::new();
        loop {
            self.editor_set_status_mssage(str::replace(format, "{}", &prompt_buffer));
            self.refresh_screen()?;

            let input_type = input.readkey()?;
            match input_type {
                Char(b'\x1b') => {
                    self.editor_set_status_mssage(String::new());
                    if let Some(cb) = callback {
                        cb(self, &prompt_buffer, b'\x1b');
                    }
                    return Ok(None);
                }
                Char(b'\r') => {
                    self.editor_set_status_mssage(String::new());
                    if let Some(cb) = callback {
                        cb(self, &prompt_buffer, b'\r');
                    }
                    return Ok(Some(prompt_buffer));
                }
                Backspace | Del => {
                    if prompt_buffer.len() > 0 {
                        prompt_buffer.pop();
                    }
                }
                Char(c) => {
                    prompt_buffer.push(char::from(c));
                    if let Some(cb) = callback {
                        cb(self, &prompt_buffer, c);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn set_control_x(&mut self, input: &mut RawMode) -> io::Result<()> {
        use crate::input::InputType::*;
        self.editor_set_status_mssage("C-x -");
        self.refresh_screen()?;

        loop {
            let input_type = input.readkey()?;
            match input_type {
                Char(b'\x1b') => {
                    self.editor_set_status_mssage("C-x esc");
                    return Ok(());
                }
                ControlS => {
                    self.editor_set_status_mssage("C-x C-s");
                    return self.save_file(input);
                }
                NoOp => {}
                _ => {
                    self.editor_set_status_mssage("Command Not Found");
                    return Ok(());
                }
            }
        }
    }

    pub fn save_file(&mut self, input: &mut RawMode) -> io::Result<()> {
        use std::fs::canonicalize;
        use std::path::Path;
        let filename;
        if self.filename.is_some() {
            filename = self.filename.clone().unwrap();
        } else {
            let result = self.editor_prompt(input, "Save as {} (ESC to cancel)", None)?;
            if let Some(f) = result {
                filename = canonicalize(Path::new(&f))?;
            } else {
                self.editor_set_status_mssage("Save aborted");
                return Ok(());
            }
        }
        let mut file_writer = BufWriter::new(File::create(&filename)?);
        let mut written_bytes = 0;
        for line in &self.content_buffer {
            file_writer.write(&format!("{}\n", &line).as_bytes())?;
            written_bytes += format!("{}\n", &line).as_bytes().len();
        }
        file_writer.flush()?;
        self.editor_set_status_mssage(format!("{} bytes written to disk", written_bytes));
        self.dirty = false;
        if self.filename.is_none() {
            self.filename = Some(filename);
        }
        Ok(())
    }

    fn editor_find_callback(&mut self, query: &str, key: u8) {
        if key == b'\r' || key == b'\x1b' {
            return;
        }
        for (i, line) in self.render_buffer.iter().enumerate() {
            if let Some(index) = line.find(&query) {
                self.cx = self.rx_to_cx(index, &self.content_buffer[i]);
                self.cy = i;
                self.row_offset = i;
                break;
            }
        }
    }

    pub fn editor_find(&mut self, input: &mut RawMode) -> io::Result<()> {
        let saved_cx = self.cx;
        let saved_cy = self.cy;
        let saved_col_offset = self.col_offset;
        let saved_row_offset = self.row_offset;
        let query = self.editor_prompt(
            input,
            "Search {} (ESC to cancel)",
            Some(Window::editor_find_callback),
        )?;
        if query.is_none() {
            self.cx = saved_cx;
            self.cy = saved_cy;
            self.col_offset = saved_col_offset;
            self.row_offset = saved_row_offset;
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

    pub fn quit(&mut self) -> io::Result<LoopStatus> {
        if self.dirty && !self.quit_confirming {
            self.editor_set_status_mssage(
                "WARNING!!! File has unsaved changed. Press Ctrl-q to quit",
            );
            self.quit_confirming = true;
            return Ok(LoopStatus::CONTINUE);
        }
        write!(self.stdout, "\x1b[2J")?;
        write!(self.stdout, "\x1b[H")?;
        self.stdout.flush()?;
        Ok(LoopStatus::STOP)
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
