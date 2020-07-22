pub enum HighlightColor {
    Normal,
    Number,
    Match,
}

pub struct Highlight {
    pub highlights: Vec<Vec<HighlightColor>>,
}

impl Highlight {
    pub fn update_row(&mut self, row_index: usize, line: &String) {
        self.highlights[row_index] = line_to_highlight_color(line);
    }

    pub fn match_row(&mut self, row_index: usize, from: usize, to: usize) {
        let current_highlight = &mut self.highlights[row_index];
        for col_index in from..to {
            current_highlight[col_index] = HighlightColor::Match;
        }
    }

    pub fn insert_row(&mut self, row_index: usize, line: &String) {
        self.highlights
            .insert(row_index, line_to_highlight_color(line));
    }

    pub fn color(&self, row_index: usize, col_index: usize) -> u8 {
        match self.highlights.get(row_index) {
            Some(row) => match row.get(col_index) {
                Some(HighlightColor::Normal) => 37,
                Some(HighlightColor::Number) => 31,
                Some(HighlightColor::Match) => 34,
                None => 39,
            },
            None => 39,
        }
    }
}

fn line_to_highlight_color(line: &String) -> Vec<HighlightColor> {
    let mut highlight_row = vec![];
    for chr in line.chars() {
        if chr.is_digit(10) {
            highlight_row.push(HighlightColor::Number);
        } else {
            highlight_row.push(HighlightColor::Normal);
        }
    }
    highlight_row
}

impl From<&Vec<String>> for Highlight {
    fn from(s: &Vec<String>) -> Self {
        let mut highlights = vec![];
        for line in s {
            highlights.push(line_to_highlight_color(&line));
        }
        Highlight { highlights }
    }
}
