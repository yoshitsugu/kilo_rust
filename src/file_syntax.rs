use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Undefined,
    C,
}

impl fmt::Display for FileType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use FileType::*;
        match *self {
            Undefined => write!(f, "undef"),
            C => write!(f, "c"),
        }
    }
}

bitflags! {
    pub struct SyntaxFlags: u16 {
      const HL_NUMBER = 1 << 8;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileSyntax {
    pub ftype: FileType,
    pub extensions: [&'static str; 5],
    pub flags: SyntaxFlags,
}

impl FileSyntax {
    pub fn new() -> FileSyntax {
        FileSyntax {
            ftype: FileType::Undefined,
            extensions: [""; 5],
            flags: SyntaxFlags::empty(),
        }
    }
}

pub static SYNTAX_DB: Lazy<HashMap<&std::ffi::OsStr, FileSyntax>> = Lazy::new(|| {
    use FileType::*;
    let mut result = HashMap::new();

    let syntaxes = vec![FileSyntax {
        ftype: C,
        extensions: ["c", "cpp", "h", "", ""],
        flags: SyntaxFlags::HL_NUMBER,
    }];
    for s in syntaxes {
        for ext in s.extensions.iter() {
            if ext.len() > 0 {
                result.insert(std::ffi::OsStr::new(ext.clone()), s);
            }
        }
    }
    result
});
