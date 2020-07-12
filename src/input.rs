use std::io::{self, stdin, Read, Write};
use std::os::unix::io::AsRawFd;

pub struct RawMode {
    pub stdin: io::Stdin,
    pub orig: termios::Termios,
}

const CTRL_Q: u8 = b'q' & 0x1f;

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

    pub fn process_keypress(&mut self) -> io::Result<()> {
        let mut one_byte: [u8; 1] = [0];
        loop {
            if self.stdin.read(&mut one_byte)? == 0 {
            } else {
                match one_byte[0] {
                    CTRL_Q => {
                        write!(io::stdout(), "\x1b[2J")?;
                        write!(io::stdout(), "\x1b[H")?;
                        io::stdout().flush()?;
                        break;
                    }
                    c => {
                        print!("{:?}\r\n", c);
                        io::stdout().flush()?;
                    }
                }
            }
        }
        Ok(())
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        termios::tcsetattr(self.stdin.as_raw_fd(), termios::TCSAFLUSH, &self.orig).unwrap();
    }
}
