use ansi_term::Colour::Red;
use ansi_term::Style;
use regex::Regex;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "trim")]
struct Opt {
    /// input file
    #[structopt(parse(from_os_str))]
    pub input: Option<PathBuf>,

    /// output file
    #[structopt(short = "o", long, parse(from_os_str))]
    pub output: Option<PathBuf>,
}

/// get an interator that reads from a file line by line
fn lines_from_file<'a>(file_name: &'a PathBuf) -> impl Iterator<Item = String> + 'a {
    let file = File::open(file_name).unwrap();
    BufReader::new(file).lines().map(Result::unwrap)
}

/// get the cleaned version of a file's content
fn handle_input_file<'a>(file_name: &PathBuf) -> Box<String> {
    // rtrim each line
    let trimmed_lines = lines_from_file(file_name)
        .map(|line| *rtrim(&line) + "\n")
        .collect::<String>();

    // remove all trailing newlines
    rtrim(&trimmed_lines)
}

fn rtrim(source: &String) -> Box<String> {
    let trailing_ws = Regex::new(r"\s+$").unwrap();
    let rtrimmed = trailing_ws.replacen(&source, 1, "").to_string();
    Box::new(rtrimmed)
}

fn visualize(file_name: &PathBuf) -> Box<String> {
    let trailing_ws = Regex::new(r"\s*$").unwrap();
    let lines = lines_from_file(file_name)
        .enumerate()
        .filter_map(|(index, line)| {
            let start_end = trailing_ws.find(&line).unwrap();
            let ws_start = start_end.start();
            let ws_end = start_end.end();
            if ws_start == ws_end {
                None
            } else {
                let ws = (ws_start..ws_end).map(|_| "_").collect::<String>();
                let ws = Style::new().on(Red).fg(Red).paint(ws);
                let line_number = index + 1;
                let keep = &line[..ws_start];
                let highlighted = format!("{}{}", keep, ws);
                Some((line_number, highlighted))
            }
        })
        .map(|(line_number, line)| {
            let num_digits = (line_number as f64).log10().floor() as i64; // TODO increase from 4
            assert!(num_digits >= 0);
            let num_padding = 4 - num_digits;
            let padding = (0..num_padding).map(|_| " ").collect::<String>();
            format!("{}{}| {}\n", padding, line_number, line)
        })
        .collect::<String>();

    rtrim(&lines)
}

fn main() {
    let opt = Opt::from_args();
    let input_file = opt.input.expect("gotta provide an input file");
    let _output_file = opt.output; // if none, print to stdout

    let output = handle_input_file(&input_file);

    // STDOUT print the cleaned output
    print!("{}\n", output);

    // STDERR print how mant bytes were removed
    let old_file_size = fs::metadata(&input_file).unwrap().len() as f64;
    let new_file_size = output.len() as f64 + 1.0;
    eprintln!(
        "\n\n{:?}: {} -> {} ({:.2}%)",
        input_file,
        old_file_size,
        new_file_size,
        new_file_size / old_file_size * 100.0
    );

    // visualize the portions of the file removed
    eprintln!("{}", visualize(&input_file));
}
