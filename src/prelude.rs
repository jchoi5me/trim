use ansi_term::Colour::Red;
use ansi_term::Style;
use regex::Regex;
use std::fmt::Display;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

const TRAILING_WS: &str = r"\s*$";

/// Return an iterator that iterates through a file line by line.
pub fn readlines<'a>(path: &'a PathBuf) -> impl Iterator<Item = String> + 'a {
    let file = File::open(path).unwrap();
    BufReader::new(file).lines().map(Result::unwrap)
}

/// Clean the given lines using the following rules:
/// 1. remove the trailing whitespace from each line
/// 2. join these line with `\n`, and remove the trailing whitespace
pub fn clean(lines: impl Iterator<Item = String>) -> Box<String> {
    let trailing_ws = Regex::new(TRAILING_WS).unwrap();
    let rtrim_n = |s: String| trailing_ws.replacen(&s, 1, "\n").to_string();
    // rtrim each line
    let trimmed_lines = lines.map(rtrim_n).collect::<String>();
    // rtrim the concat result
    Box::new(rtrim_n(trimmed_lines))
}

fn red_padding_with_len(length: usize) -> impl Display {
    let padding = (0..length).map(|_| "_").collect::<String>();
    Style::new().on(Red).fg(Red).paint(padding)
}

///
pub fn visualize(path: &PathBuf, trimmed: &Box<String>) -> Box<String> {
    let lines: Vec<String> = readlines(path).collect();
    let trimmed_lines: Vec<String> = trimmed.split("\n").map(String::from).collect();

    let header = (0..1).map(|_| format!("{:>6}|{:?}\n", "file", path));
    let body_lines = lines
        .iter()
        .zip(trimmed_lines.iter())
        .enumerate()
        .map(|(i, (j, k))| (i + 1, (j, k)))
        .filter(|(_, (line, trimmed))| line.len() != trimmed.len())
        .map(|(line_number, (line, trimmed))| {
            let padding_length = line.len() - trimmed.len();
            let padding = red_padding_with_len(padding_length);
            format!("{:>6}|{}{}\n", line_number, trimmed, padding)
        });

    let tail_lines = lines
        .iter()
        .enumerate()
        .skip(trimmed_lines.len())
        .map(|(i, j)| (i + 1, j))
        .map(|(line_number, line)| {
            let padding_length = line.len();
            let padding = red_padding_with_len(padding_length);
            format!("{:>6}|{}\n", line_number, padding)
        });

    Box::new(header.chain(body_lines).chain(tail_lines).collect())
}
