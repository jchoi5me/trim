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

fn lines_from_file<'a>(file_name: &'a PathBuf) -> impl Iterator<Item = String> + 'a {
    let file = File::open(file_name).unwrap();
    BufReader::new(file).lines().map(Result::unwrap)
}

fn handle_input_file<'a>(file_name: &PathBuf) -> Box<String> {
    let lines = lines_from_file(file_name);
    trim_lines(lines)
}

fn trim_lines(lines: impl Iterator<Item = String>) -> Box<String> {
    let trailing_ws = Regex::new(r"\s+$").unwrap();
    let trimmed_lines = lines
        .map(|line| trailing_ws.replacen(&line, 1, "").to_string() + "\n")
        .collect::<String>();

    Box::new(trailing_ws.replacen(&trimmed_lines, 1, "").to_string())
}

fn visualize(file_name: &PathBuf) -> Box<String> {
    let trailing_ws = Regex::new(r"\s*$").unwrap();
    let lines = lines_from_file(file_name)
        .enumerate()
        .filter_map(|(i, line)| {
            let start_end = trailing_ws.find(&line).unwrap();
            let ws_start = start_end.start();
            let ws_end = start_end.end();
            match ws_start == ws_end {
                true => None,
                false => {
                    let ws = (ws_start..ws_end).map(|_| "_").collect::<String>();
                    let ws = Style::new().on(Red).fg(Red).paint(ws);
                    let line_number = i + 1;
                    let keep = &line[..ws_start];
                    let highlighted = format!("{}{}", keep, ws);
                    Some((line_number, highlighted))
                }
            }
        })
        .map(|(i, line)| {
            let num_padding = 4 - (i / 10);
            let padding = (0..num_padding).map(|_| " ").collect::<String>();
            format!("{}{}| {}", padding, i, line)
        })
        .collect::<Vec<String>>()
        .join("\n");

    Box::new(trailing_ws.replacen(&lines, 1, "").to_string())
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
