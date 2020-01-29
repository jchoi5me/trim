use ansi_term::Colour::Red;
use ansi_term::Colour::White;
use ansi_term::Style;
use regex::Regex;
use std::fmt::Debug;
use std::fmt::Display;
use std::fs::File;
use std::io::stderr;
use std::io::stdin;
use std::io::stdout;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Write;
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

fn handle<I>(lines: I) -> Result<(), Error>
where
    I: Iterator<Item = Result<String, Error>>,
{
    handle_custom(lines, &mut stdout().lock(), &mut stderr().lock())
}

fn handle_custom<I, W1, W2>(lines: I, out: &mut W1, err: &mut W2) -> Result<(), Error>
where
    I: Iterator<Item = Result<String, Error>>,
    W1: Write,
    W2: Write,
{
    let t_ws = Regex::new(r"\s*$").map_err(io_err)?;
    let rtrim_w = |src: &str| t_ws.replace(src, "").to_string();

    lines
        .map(Result::unwrap)
        .enumerate()
        .map(|(index, line)| (index + 1, line))
        .map(|(line_number, line)| {
            let trimmed_line = rtrim_w(&line);
            let opt_visual = Some(line.len() - trimmed_line.len())
                .filter(|x| x > &0)
                .map(red_padding_with_len)
                .map(|padding| format!("{:>6}|{}{}", line_number, trimmed_line, padding));
            (trimmed_line, opt_visual)
        })
        .fold(
            Ok(0usize),
            |opt_lf_count: Result<usize, Error>, (line, opt_vis)| match &opt_lf_count {
                Ok(count) if line.len() == 0 => Ok(count + 1),
                Ok(count) => {
                    let lfs: String = (0..*count).map(|_| "\n").collect();
                    write!(out, "{}", lfs)?; // accumulated line feeds
                    write!(out, "{}\n", line)?; // actual line
                    if let Some(vis) = opt_vis {
                        write!(err, "{}\n", vis)?;
                    }
                    Ok(0)
                }
                Err(_) => opt_lf_count,
            },
        )?;

    out.flush()?;
    err.flush()?;
    Ok(())
}

fn main() {
    let args = Opt::from_args();

    let (opt_path, use_stdin) = match args.file {
        Some(path) if path.to_str() == Some("-") => (None, true),
        Some(path) => (Some(path), false),
        None => (None, true),
    };

    let result = match use_stdin {
        true => handle(stdin().lock().lines()),
        false => handle(readlines(&opt_path.unwrap()).unwrap()),
    };
    match result {
        Ok(_) => (),
        Err(err) => eprintln!("{}", err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rayon::prelude::*;

    fn test_data() -> Vec<(&'static str, &'static str)> {
        vec![
            ("", ""),
            ("abc", "abc\n"),
            ("   absoi ", "   absoi\n"),
            ("ab \ncd \n  \n\n  \n", "ab\ncd\n"),
        ]
    }

    #[test]
    fn parametrized() {
        test_data().into_par_iter().for_each(|(input, expected)| {
            let lines = input.split('\n').map(String::from).map(|s| Ok(s));
            let mut out = Vec::new();
            let mut err = Vec::new();

            handle_custom(lines, &mut out, &mut err).unwrap();

            assert_eq!(expected.as_bytes(), &out[..]);
        });
    }
}
