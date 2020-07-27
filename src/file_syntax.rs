use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Undefined,
    C,
    Rust,
}

impl fmt::Display for FileType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use FileType::*;
        match *self {
            Undefined => write!(f, "--"),
            C => write!(f, "C"),
            Rust => write!(f, "Rust"),
        }
    }
}

bitflags! {
    pub struct SyntaxFlags: u16 {
      const HL_NUMBER = 1 << 0;
      const HL_STRING = 1 << 1;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileSyntax {
    pub ftype: FileType,
    pub extensions: &'static [&'static str],
    pub singleline_comment_start: &'static str,
    pub multiline_comment_start: &'static str,
    pub multiline_comment_end: &'static str,
    pub keywords: &'static [&'static str],
    pub flags: SyntaxFlags,
}

impl FileSyntax {
    pub fn new() -> FileSyntax {
        FileSyntax {
            ftype: FileType::Undefined,
            extensions: &[],
            singleline_comment_start: "#",
            multiline_comment_start: "",
            multiline_comment_end: "",
            keywords: &[],
            flags: SyntaxFlags::empty(),
        }
    }
}
const C_EXTENSIONS: [&'static str; 3] = ["c", "cpp", "h"];

const C_KEYWEORDS: [&'static str; 23] = [
    "switch",
    "if",
    "while",
    "for",
    "break",
    "continue",
    "return",
    "else",
    "struct",
    "union",
    "typedef",
    "static",
    "enum",
    "class",
    "case",
    "int|",
    "long|",
    "double|",
    "float|",
    "char|",
    "unsigned|",
    "signed|",
    "void|",
];

const RUST_EXTENSIONS: [&'static str; 1] = ["rs"];

const RUST_KEYWEORDS: [&'static str; 37] = [
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false|", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref|", "return",
    "self|", "Self|", "static", "struct", "super", "trait", "true|", "type", "unsafe", "use",
    "where", "while", "async", "await",
];

pub static SYNTAX_DB: Lazy<HashMap<&std::ffi::OsStr, FileSyntax>> = Lazy::new(|| {
    use FileType::*;
    let mut result = HashMap::new();

    let syntaxes = vec![
        FileSyntax {
            ftype: C,
            extensions: &C_EXTENSIONS,
            singleline_comment_start: "//",
            multiline_comment_start: "/*",
            multiline_comment_end: "*/",
            keywords: &C_KEYWEORDS,
            flags: SyntaxFlags::HL_NUMBER | SyntaxFlags::HL_STRING,
        },
        FileSyntax {
            ftype: Rust,
            extensions: &RUST_EXTENSIONS,
            singleline_comment_start: "//",
            multiline_comment_start: "/*",
            multiline_comment_end: "*/",
            keywords: &RUST_KEYWEORDS,
            flags: SyntaxFlags::HL_NUMBER | SyntaxFlags::HL_STRING,
        },
    ];
    for s in syntaxes {
        for ext in s.extensions.iter() {
            result.insert(std::ffi::OsStr::new(ext.clone()), s);
        }
    }
    result
});
