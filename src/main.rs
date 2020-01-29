use ansi_term::Colour::Red;
use ansi_term::Colour::White;
use ansi_term::Style;
use regex::Regex;
use std::fmt::Debug;
use std::fmt::Display;
use std::fs::File;
use std::io::stdin;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Error;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "trim")]
struct Opt {
    /// Files to trim. If empty or is '-', stdin will be used to grab lines.
    #[structopt(parse(from_os_str))]
    pub file: Option<PathBuf>,
}

#[inline]
fn io_err<D>(error: D) -> Error
where
    D: Debug,
{
    Error::new(ErrorKind::Other, format!("{:?}", error))
}

/// Return an iterator that iterates through a file line by line.
fn readlines(path: &Path) -> Result<impl Iterator<Item = Result<String, Error>>, Error> {
    File::open(path).map(BufReader::new).map(BufReader::lines)
}

fn red_padding_with_len(length: usize) -> impl Display {
    let padding: String = (0..length).map(|_| "_").collect();
    Style::new().on(Red).fg(White).paint(padding)
}

fn handle_iter<I>(lines: I) -> Result<(), Error>
where
    I: Iterator<Item = Result<String, Error>>,
{
    let t_ws = Regex::new(r"\s*$").map_err(io_err)?;
    let rtrim_w = |src: &str, w: &str| t_ws.replace(&src, w).to_string();

    let newline_count = lines
        .map(Result::unwrap)
        .enumerate()
        .map(|(index, line)| (index + 1, line))
        .map(|(line_number, line)| {
            let trimmed_line = rtrim_w(&line, "");
            let padding_length = match trimmed_line.len() < line.len() {
                true => line.len() - trimmed_line.len(),
                false => 0,
            };
            let opt_visual_line = match padding_length > 0 {
                true => {
                    let padding = red_padding_with_len(padding_length);
                    Some(format!(
                        "{:>6}|{}{}",
                        line_number,
                        rtrim_w(&trimmed_line, ""),
                        padding
                    ))
                }
                false => None,
            };

            (trimmed_line, opt_visual_line)
        })
        .fold((0usize), |newline_count, (line, opt_vis)| {
            match line.len() {
                0 => newline_count + 1,
                _ => {
                    let newlines: String = (0..newline_count).map(|_| "\n").collect();
                    print!("{}", newlines);
                    println!("{}", line);
                    if let Some(vis) = opt_vis {
                        eprintln!("{}", vis);
                    }
                    0
                }
            }
        });

    Ok(())
}

fn main() {
    let args = Opt::from_args();

    let path = args.file.unwrap();
    let use_stdin = false;

    if use_stdin {
        handle_iter(stdin().lock().lines());
    } else {
        match readlines(&path) {
            Ok(lines) => match handle_iter(lines) {
                Ok(result) => println!("{:?}", result),
                Err(err) => eprintln!("{:?}", err),
            },
            Err(err) => eprintln!("{:?}", err),
        }
    };
}
