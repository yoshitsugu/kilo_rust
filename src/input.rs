use crate::window::Window;
use std::io::{self, stdin, Read, Write};
use std::os::unix::io::AsRawFd;

pub enum CursorMoveDirection {
    Left,
    Right,
    Up,
    Down,
    Top,
    Bottom,
    LineTop,
    LineBottom,
}
pub enum InputType {
    CursorMove(CursorMoveDirection),
    Char(u8),
    Del,
}

pub struct RawMode {
    pub stdin: io::Stdin,
    pub orig: termios::Termios,
}

pub const CTRL_Q: u8 = b'q' & 0x1f;
pub const CTRL_N: u8 = b'n' & 0x1f;
pub const CTRL_B: u8 = b'b' & 0x1f;
pub const CTRL_F: u8 = b'f' & 0x1f;
pub const CTRL_P: u8 = b'p' & 0x1f;
pub const CTRL_A: u8 = b'a' & 0x1f;
pub const CTRL_E: u8 = b'e' & 0x1f;

pub enum LoopStatus {
    CONTINUE,
    STOP,
}

impl RawMode {
    pub fn new() -> Result<RawMode, io::Error> {
        use termios::*;

        let stdin = stdin();
        let stdin_fd = stdin.as_raw_fd();
        let mut termios = Termios::from_fd(stdin_fd)?;
        let orig = termios;

        termios::tcgetattr(stdin_fd, &mut termios)?;
        termios.c_iflag &= !(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
        termios.c_oflag &= !(OPOST);
        termios.c_cflag |= CS8;
        termios.c_lflag &= !(ECHO | ICANON | IEXTEN | ISIG);
        termios.c_cc[VMIN] = 0;
        termios.c_cc[VTIME] = 1;
        termios::tcsetattr(stdin_fd, TCSAFLUSH, &mut termios)?;
        Ok(RawMode { stdin, orig })
    }

    pub fn readkey(&mut self) -> io::Result<InputType> {
        use CursorMoveDirection::*;
        use InputType::*;
        let mut seq: [u8; 4] = [0; 4];
        if self.stdin.read(&mut seq)? > 0 {
            if seq[0] == b'\x1b' {
                if seq[1] == b'[' {
                    println!("seq: {}, {}", seq[2], seq[3]);
                    if seq[2] >= b'0' && seq[2] <= b'9' && seq[3] == b'~' {
                        return match seq[2] {
                            b'1' => Ok(CursorMove(LineTop)),    // Homeキー
                            b'3' => Ok(Del),                    // Delキー
                            b'4' => Ok(CursorMove(LineBottom)), // Endキー
                            b'5' => Ok(CursorMove(Top)),        // PageUpキー
                            b'6' => Ok(CursorMove(Bottom)),     // PageDownキー
                            b'7' => Ok(CursorMove(LineTop)),    // Homeキー
                            b'8' => Ok(CursorMove(LineBottom)), // Endキー
                            _ => Ok(Char(b'\x1b')),
                        };
                    } else {
                        return match seq[2] {
                            b'A' => Ok(CursorMove(Up)),         // ↑キー
                            b'B' => Ok(CursorMove(Down)),       // ↓キー
                            b'C' => Ok(CursorMove(Right)),      // →キー
                            b'D' => Ok(CursorMove(Left)),       // ←キー
                            b'H' => Ok(CursorMove(LineTop)),    // Homeキー
                            b'F' => Ok(CursorMove(LineBottom)), // Endキー
                            _ => Ok(Char(b'\x1b')),
                        };
                    }
                } else if seq[1] == b'O' {
                    return match seq[2] {
                        b'H' => Ok(CursorMove(LineTop)),    // Homeキー
                        b'F' => Ok(CursorMove(LineBottom)), // Endキー
                        _ => Ok(Char(b'\x1b')),
                    };
                }
                return Ok(Char(b'\x1b'));
            } else {
                return match seq[0] {
                    CTRL_P => Ok(CursorMove(Up)),
                    CTRL_N => Ok(CursorMove(Down)),
                    CTRL_F => Ok(CursorMove(Right)),
                    CTRL_B => Ok(CursorMove(Left)),
                    CTRL_A => Ok(CursorMove(LineTop)),
                    CTRL_E => Ok(CursorMove(LineBottom)),
                    c => Ok(Char(c)),
                };
            }
        }
        Ok(Char(b'\x1b'))
    }

    pub fn process_keypress(&mut self, window: &mut Window) -> io::Result<LoopStatus> {
        use InputType::*;
        let input_type = self.readkey()?;
        match input_type {
            Char(b'\x1b') => {}
            Char(CTRL_Q) => {
                write!(io::stdout(), "\x1b[2J")?;
                write!(io::stdout(), "\x1b[H")?;
                io::stdout().flush()?;
                return Ok(LoopStatus::STOP);
            }
            CursorMove(d) => window.move_cursor(d),
            Del => {
                print!("Del key pressed\r\n");
                io::stdout().flush()?;
            }
            Char(c) => {
                print!("{:?}\r\n", c);
                io::stdout().flush()?;
            }
        }
        Ok(LoopStatus::CONTINUE)
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        termios::tcsetattr(self.stdin.as_raw_fd(), termios::TCSAFLUSH, &self.orig).unwrap();
    }
}