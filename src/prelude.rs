use ansi_term::Colour::Red;
use ansi_term::Style;
use regex::Regex;
use std::fmt::Display;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

pub struct TrimResult {
    /// cleaned output mentioned above
    pub trimmed: Box<String>,
    /// visualization of the cleaning process
    pub visualized: Box<String>,
    /// number of bytes saved through the cleaning
    pub saved_bytes: usize,
}

/// Return an iterator that iterates through a file line by line.
pub fn readlines(path: &PathBuf) -> impl Iterator<Item = String> {
    let file = File::open(path).unwrap();
    BufReader::new(file).lines().map(Result::unwrap)
}

/// Clean the given lines using the following rules:
/// 1. remove the trailing whitespace from each line
/// 2. join these line with `\n`, and replace the trailing whitespace with `\n`
pub fn clean(lines: impl Iterator<Item = String>) -> TrimResult {
    let t_ws = Regex::new(r"\s*$").unwrap();
    let rtrim_w = |src: &String, w: &str| t_ws.replace(&src, w).to_string();

    let result: Vec<(String, Option<String>, usize)> = lines
        .enumerate()
        .map(|(index, line)| (index + 1, line)) // 1-base now
        .map(|(line_number, line)| {
            let trimmed_line = rtrim_w(&line, "\n");
            let padding_length = match trimmed_line.len() < line.len() {
                true => line.len() - trimmed_line.len(),
                false => 0,
            };
            let visual_line = match padding_length > 0 {
                true => {
                    let padding = red_padding_with_len(padding_length);
                    Some(format!(
                        "{:>6}|{}{}\n",
                        line_number,
                        rtrim_w(&trimmed_line, ""),
                        padding
                    ))
                }
                false => None,
            };

            (trimmed_line, visual_line, padding_length)
        })
        .collect();

    let lines_trimmed: String = result.iter().cloned().map(|t| t.0).collect();
    let end_trimmed = rtrim_w(&lines_trimmed, "\n");
    let visual = result
        .iter()
        .cloned()
        .map(|t| t.1)
        .filter(Option::is_some)
        .map(Option::unwrap)
        .collect();

    TrimResult {
        trimmed: Box::new(end_trimmed),
        visualized: Box::new(visual),
        saved_bytes: result.iter().map(|t| &t.2).sum(),
    }
}

fn red_padding_with_len(length: usize) -> impl Display {
    let padding: String = (0..length).map(|_| "_").collect();
    Style::new().on(Red).fg(Red).paint(padding)
}
