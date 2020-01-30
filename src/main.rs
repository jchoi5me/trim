use ansi_term::Colour::Red;
use ansi_term::Colour::White;
use ansi_term::Style;
use regex::Regex;
use std::cmp::max;
use std::fmt::Debug;
use std::fmt::Display;
use std::fs::File;
use std::io;
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
    /// file to trim; if '-' or not provided, stdin will be used
    #[structopt(parse(from_os_str))]
    pub file: Option<PathBuf>,
}

/// This is used to map `Err(err: D)` to `Err(std::io::Error)`.
///
/// # Returns
///
/// `std::io::Error` with `format!("{:?}", error)` as its content.
#[inline]
fn io_err<D>(error: D) -> Error
where
    D: Debug,
{
    Error::new(ErrorKind::Other, format!("{:?}", error))
}

/// # Returns
///
/// An iterator that reads through the file under `path` line by line.
#[inline]
fn readlines(path: &Path) -> io::Result<impl Iterator<Item = io::Result<String>>> {
    File::open(path).map(BufReader::new).map(BufReader::lines)
}

/// Used to visualize the trimmed whitespace.
///
/// # Returns
///
/// Displayable struct, resulting in `length` chars with a red background and a white foreground,
/// each with `'-'` as the text.
#[inline]
fn red_padding_with_len(length: usize) -> impl Display {
    Style::new()
        .on(Red)
        .fg(White)
        .paint((0..length).map(|_| '_').collect::<String>())
}

/// Call `handle_custom` with `stdout` as `out`, and `stderr` as `err`.
///
/// # Parameters
///
/// 1. `lines`: lines of file to trim
#[inline]
fn handle<I>(lines: I) -> io::Result<()>
where
    I: Iterator<Item = io::Result<String>>,
{
    handle_custom(lines, &mut stdout().lock(), &mut stderr().lock())
}

fn handle_custom<I, W1, W2>(lines: I, out: &mut W1, err: &mut W2) -> io::Result<()>
where
    I: Iterator<Item = io::Result<String>>,
    W1: Write,
    W2: Write,
{
    let t_ws = Regex::new(r"\s*$").map_err(io_err)?;
    let rtrim_w = |src: &str| t_ws.replace(src, "").to_string();

    // lf_count == number of trailing newlines
    // lf_count + u8_saved == total number of bytes trimmed
    let (lf_count, u8_saved) = lines
        .map(io::Result::unwrap)
        .enumerate()
        .map(|(index, line)| (index + 1, line))
        .map(|(line_number, line)| {
            let trimmed_line = rtrim_w(&line);
            let length_diff = max(0, line.len() - trimmed_line.len());
            let opt_visual = Some(length_diff)
                .filter(|x| x > &0)
                .map(red_padding_with_len)
                .map(|red_pad| format!("{:>6}|{}{}", line_number, trimmed_line, red_pad));
            (trimmed_line, opt_visual, length_diff)
        })
        .fold(
            Ok((0usize, 0usize)), // (number of `\n` to print, total number of bytes trimmed)
            |results: io::Result<(usize, usize)>, (line, opt_visual, u8_trimmed)| {
                match &results {
                    Ok((count, total)) if line.len() == 0 => Ok((count + 1, total + u8_trimmed)), // empty line encountered
                    Ok((count, total)) => {
                        // print the accumulated newlines, if any
                        let lfs: String = (0..*count).map(|_| '\n').collect();
                        write!(out, "{}", lfs)?;
                        // print the actual line
                        writeln!(out, "{}", line)?;

                        // print the visual to err, if any
                        match opt_visual {
                            Some(visual) => {
                                writeln!(err, "{}", visual)?;
                                Ok((0, total + u8_trimmed))
                            }
                            None => Ok((0, *total)), // no `\n` to print
                        }
                    }
                    Err(_) => results,
                }
            },
        )?;

    writeln!(err, "\n{:>6} trailing `\\n` trimmed", lf_count)?;
    writeln!(err, "{:>6} bytes saved overall", u8_saved + lf_count)?;

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
        test_data().into_iter().for_each(|(input, expected)| {
            let lines = input.split('\n').map(String::from).map(|s| Ok(s));
            let mut out = Vec::new();
            let mut err = Vec::new();

            handle_custom(lines, &mut out, &mut err).unwrap();

            assert_eq!(expected.as_bytes(), &out[..]);
        });
    }
}
