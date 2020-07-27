use crate::file_syntax::{FileSyntax, FileType, SyntaxFlags, SYNTAX_DB};
use std::path::PathBuf;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum HighlightColor {
    Normal,
    Number,
    String,
    Comment,
    MultilineComment,
    Keyword1,
    Keyword2,
    Match,
}

pub struct Highlight {
    pub syntax: FileSyntax,
    pub highlights: Vec<Vec<HighlightColor>>,
    pub in_comment: Vec<bool>,
}

fn get_syntax(path: PathBuf) -> FileSyntax {
    match SYNTAX_DB.get(path.extension().unwrap_or(std::ffi::OsStr::new(""))) {
        Some(syntax) => *syntax,
        None => FileSyntax::new(),
    }
}

impl Highlight {
    pub fn new(s: &Vec<String>, path: PathBuf) -> Self {
        let syntax = get_syntax(path);
        let mut h = Highlight {
            syntax,
            highlights: vec![],
            in_comment: vec![],
        };
        for (index, line) in s.into_iter().enumerate() {
            h.highlights.push(vec![]);
            h.in_comment.push(false);
            match h.line_to_highlight_color(&line, index) {
                (row, _) => h.highlights[index] = row,
            }
        }
        h
    }

    pub fn update_row(&mut self, row_index: usize, line: &String) -> Option<usize> {
        match self.line_to_highlight_color(line, row_index) {
            (row, Some(need_to_update_index)) => {
                self.highlights[row_index] = row;
                Some(need_to_update_index)
            }
            (row, None) => {
                self.highlights[row_index] = row;
                None
            }
        }
    }

    pub fn match_row(&mut self, row_index: usize, from: usize, to: usize) {
        let current_highlight = &mut self.highlights[row_index];
        for col_index in from..to {
            current_highlight[col_index] = HighlightColor::Match;
        }
    }

    pub fn insert_row(&mut self, row_index: usize, line: &String) -> Option<usize> {
        self.highlights.insert(row_index, vec![]);
        self.in_comment.push(false);
        match self.line_to_highlight_color(line, row_index) {
            (row, Some(need_to_update_index)) => {
                self.highlights[row_index] = row;
                Some(need_to_update_index)
            }
            (row, None) => {
                self.highlights[row_index] = row;
                None
            }
        }
    }

    pub fn remove_row(&mut self, row_index: usize) {
        self.highlights.remove(row_index);
        self.in_comment.remove(row_index);
    }

    pub fn color(&self, row_index: usize, col_index: usize) -> u8 {
        match self.highlights.get(row_index) {
            Some(row) => match row.get(col_index) {
                Some(HighlightColor::Normal) => 37,
                Some(HighlightColor::Number) => 31,
                Some(HighlightColor::String) => 35,
                Some(HighlightColor::Comment) => 36,
                Some(HighlightColor::MultilineComment) => 36,
                Some(HighlightColor::Keyword1) => 33,
                Some(HighlightColor::Keyword2) => 32,
                Some(HighlightColor::Match) => 34,
                None => 39,
            },
            None => 39,
        }
    }

    fn line_to_highlight_color(
        &mut self,
        line: &String,
        row_index: usize,
    ) -> (Vec<HighlightColor>, Option<usize>) {
        let mut highlight_row = vec![];
        let mut prev_sep = true;
        let mut in_string: Option<char> = None;
        let mut in_comment = row_index > 0 && self.in_comment[row_index - 1];
        let mut skip = 0;
        let scs = self.syntax.singleline_comment_start;
        let mcs = self.syntax.multiline_comment_start;
        let mce = self.syntax.multiline_comment_end;
        for (ci, chr) in line.chars().enumerate() {
            if self.syntax.ftype == FileType::Undefined {
                highlight_row.push(HighlightColor::Normal);
                continue;
            }
            if skip > 0 {
                skip -= 1;
                continue;
            }
            let prev_hl = if ci == 0 {
                HighlightColor::Normal
            } else {
                highlight_row[ci - 1]
            };

            // Single line comment
            if scs.len() > 0
                && in_string.is_none()
                && !in_comment
                && line.len() > scs.len()
                && ci < line.len() - scs.len()
            {
                if &line[ci..ci + scs.len()] == scs {
                    for _ in 0..line.len() - ci {
                        highlight_row.push(HighlightColor::Comment);
                    }
                    break;
                }
            }

            // Multiline comment
            if mcs.len() > 0 && mce.len() > 0 && in_string.is_none() {
                if in_comment {
                    highlight_row.push(HighlightColor::MultilineComment);
                    if let Some(chars) = &line.get(ci..ci + mce.len()) {
                        if chars == &mce {
                            for _ in 1..mce.len() {
                                highlight_row.push(HighlightColor::MultilineComment);
                            }
                            skip = mce.len() - 2;
                            in_comment = false;
                            prev_sep = true;
                            continue;
                        }
                    }
                    continue;
                } else {
                    if let Some(chars) = &line.get(ci..ci + mcs.len()) {
                        if chars == &mcs {
                            for _ in 0..mcs.len() {
                                highlight_row.push(HighlightColor::MultilineComment);
                            }
                            skip = mcs.len() - 1;
                            in_comment = true;
                            continue;
                        }
                    }
                }
            }

            // String
            if (self.syntax.flags & SyntaxFlags::HL_STRING).bits() != 0 {
                match in_string {
                    Some(quotation) => {
                        highlight_row.push(HighlightColor::String);
                        if chr == '\\' && ci + 1 < line.len() {
                            highlight_row.push(HighlightColor::String);
                            skip = 1;
                            continue;
                        }
                        if quotation == chr {
                            in_string = None;
                        }
                        prev_sep = true;
                        continue;
                    }
                    None => {
                        if chr == '"' || chr == '\'' {
                            in_string = Some(chr);
                            highlight_row.push(HighlightColor::String);
                            continue;
                        }
                    }
                }
            }

            // Number
            if (self.syntax.flags & SyntaxFlags::HL_NUMBER).bits() != 0 {
                if (chr.is_digit(10) && (prev_sep || prev_hl == HighlightColor::Number))
                    || (chr == '.' && prev_hl == HighlightColor::Number)
                {
                    highlight_row.push(HighlightColor::Number);
                    prev_sep = false;
                    continue;
                }
            }

            // Keyword
            if prev_sep {
                for keyword in self.syntax.keywords {
                    let mut is_kw2 = false;
                    let mut kw = *keyword;
                    if keyword.ends_with("|") {
                        kw = &keyword[0..keyword.len() - 1];
                        is_kw2 = true;
                    }
                    if line[ci..].len() < kw.len() + 1 {
                        continue;
                    }
                    if &line[ci..ci + kw.len()] == kw
                        && is_separator(line.chars().nth(ci + kw.len()).unwrap())
                    {
                        for _ in 0..kw.len() {
                            if is_kw2 {
                                highlight_row.push(HighlightColor::Keyword2);
                            } else {
                                highlight_row.push(HighlightColor::Keyword1);
                            }
                        }
                        skip = kw.len() - 1;
                        break;
                    }
                }
                if skip > 0 {
                    prev_sep = false;
                    continue;
                }
            }
            highlight_row.push(HighlightColor::Normal);
            prev_sep = is_separator(chr);
        }

        let current_in_comment = self.in_comment[row_index].clone();
        if in_comment != current_in_comment {
            self.in_comment[row_index] = in_comment;
            (highlight_row, Some(row_index + 1))
        } else {
            (highlight_row, None)
        }
    }
}

fn is_separator(chr: char) -> bool {
    return chr.is_whitespace() || chr == '\0' || ",.()+-/*=~%<>[];".contains(chr);
}
