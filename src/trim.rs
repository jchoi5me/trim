use rayon::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::fs::copy;
use std::fs::remove_file;
use std::fs::rename;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::stderr;
use std::io::stdout;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use crate::util::*;

/// Summary of everything that happened during the trim.
pub struct TrimResult {
    pub bytes_saved: i32,
}

/// Trim the lines in `Iterator` and write them to `std::io::Stdout`.
///
/// # Parameters
///
/// 1. `lines` -- iterator of lines to trim_iter
/// 1. `suppress_visual` -- if `false`, write visuals to `std::io::Stderr`, don't otherwise
/// 1. `suppress_newline` -- if `false`, end the last line with `\n`, don't otherwise
///
/// # Returns
///
/// See `TrimResult`.
///
/// # Side Effects
///
/// 1. trimmed lines are written to `std::io::Stdout`
/// 1. visualizations are written to `std::io::Stderr` if and only if `suppress_visual` is `false`
#[inline]
pub fn trim_iter<I>(
    lines: I,
    suppress_visual: bool,
    suppress_newline: bool,
) -> io::Result<TrimResult>
where
    I: Iterator<Item = io::Result<String>>,
{
    let err = stderr(); // declare outside the `match` to circumvent the borrow checker

    let bytes_saved = trim_custom(
        lines,
        &mut stdout().lock(),
        &mut match suppress_visual {
            true => None,
            false => Some(err.lock()),
        },
        suppress_newline,
    )?;

    Ok(TrimResult { bytes_saved })
}

/// Trim the lines in each file in `files`, in-place.
///
/// # Parameters
///
/// 1. `files` -- files to trim, in-place
/// 1. `suppress_newline` -- if `false`, end the last line with `\n`, don't otherwise
///
/// # Returns
///
/// Mapping,
/// - from: a path to the file being trimmed in-place
/// - to: the result of trimming that file
///
/// # Side Effects
///
/// The content of each file in `files` is overwritten with its trimmed content, if the trimmed
/// content differs from the original content. This overwriting happens atomically.
pub fn trim_files(
    files: &Vec<PathBuf>,
    suppress_newline: bool,
) -> HashMap<PathBuf, io::Result<TrimResult>> {
    files
        .into_par_iter()
        .map(|path_buf| (path_buf.clone(), trim_file(&path_buf, suppress_newline)))
        .collect()
}

/// Like `trim_files`, but for a single file.
fn trim_file(path: &Path, suppress_newline: bool) -> io::Result<TrimResult> {
    // create a tempfile to hold the trimmed content
    let basename = path.file_name().unwrap().to_str().unwrap().to_string();
    let basename = format!("{}.trim", hash_default(&basename));
    let copy_path = env::temp_dir().as_path().join(basename);
    if copy_path.exists() {
        remove_file(&copy_path)?;
    }
    copy(path, &copy_path)?; // copy contents and permissions

    // open a file with write permission, overwriting its content
    let mut copy_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&copy_path)?;

    // actual trimming
    let bytes_saved = trim_custom(
        readlines(path)?,
        &mut copy_file,
        &mut None::<File>,
        suppress_newline,
    )?;

    rename(copy_path, path)?; // mv --force "$copy_path" "$path"

    Ok(TrimResult { bytes_saved })
}

/// # Parameters
///
/// 1. `lines` -- lines to trim, as an iterator
/// 1. `out` -- where trimmed results will be written
/// 1. `err_opt` -- where visualizations of the trim will be written, optional
/// 1. `suppress_newline` -- omit `\n` at the end of the last line if true, put it otherwise
///
/// # Returns
///
/// Number of bytes trimmed.
///
/// # Side Effects
///
/// 1. trimmed lines are written to `out`
/// 1. `if let Some(err) = err_opt`, visualizations of the trimmings are written to `err`
fn trim_custom<I, W, E>(
    lines: I,
    out: &mut W,
    err_opt: &mut Option<E>,
    suppress_newline: bool,
) -> io::Result<i32>
where
    I: Iterator<Item = io::Result<String>>,
    W: Write,
    E: Write,
{
    // regex for contiguous whitespace at the end of a line
    let trailing_ws = Regex::new(r"\s*$").unwrap();

    // `lf_trimmed` = number of linebreaks encountered, but not written yet
    // `u8_trimmed` = number of bytes trimmed for sure
    //
    // contains lots of hacks in order to do the trimming in a streaming style
    let (lf_trimmed, u8_trimmed) = lines
        .map(io::Result::unwrap)
        .enumerate()
        .map(|(index, line)| (index + 1, line) /* make 1-based */)
        .map(|(line_number, line)| {
            let trimmed_line = trailing_ws.replace(&line, "").to_string(); // remove `\s*$`
            let bytes_saved = line.len() - trimmed_line.len();
            let visual_opt = Some(bytes_saved)
                .filter(|x| x > &0)
                .map(red_padding_with_len)
                .map(|red_pad| format!("{:>6}|{}{}", line_number, trimmed_line, red_pad));
            (trimmed_line, visual_opt, bytes_saved) // (String, Option<impl >
        })
        .fold(
            // same type as `(lf_trimmed, u8_trimmed)`
            io::Result::Ok((0usize, 0usize)),
            |acc, (trimmed_line, opt_visual, u8_trimmed)| {
                match &acc {
                    // empty line encountered; increment the `lf_count` without writing, because if
                    // this `\n` is one of the trailing newlines in the file, we don't want
                    // to print it and include it as bytes saved, so defer the printing until later
                    Ok((lf_count, total)) if trimmed_line.len() == 0 => {
                        Ok((lf_count + 1, total + u8_trimmed))
                    }
                    // most common case; a non-empty line
                    Ok((lf_count, total)) => {
                        // print the accumulated newlines, if any
                        let lfs: String = (0..*lf_count).map(|_| '\n').collect();
                        write!(out, "{}{}", lfs, trimmed_line)?;

                        // print the visual to err, if applicable
                        if let Some(err) = err_opt {
                            if let Some(visual) = opt_visual {
                                writeln!(err, "{}", visual)?;
                            }
                        }
                        // `\n` may or may not exist at the end of this line, but pretend like it
                        // exists for now, and defer the printing until later
                        Ok((1, total + u8_trimmed))
                    }
                    // just propagate the err
                    Err(_) => acc,
                }
            },
        )?;

    // trailing `\n` is not printed in `fold`, so if `\n` is not to be suppressed then print one now
    if !suppress_newline {
        write!(out, "\n")?;
    }

    // flush both out and err
    out.flush()?;
    if let Some(err) = err_opt {
        err.flush()?;
    }

    // total number of bytes saved
    let bytes_saved = (u8_trimmed + lf_trimmed) as i32
        // `lf_trimmed` includes an imaginary `\n` that may or may not exist
        + match lf_trimmed {
            // this means that the last line is nonempty and may or may not end with `\n`
            // as mentioned in the tests, `abc\n` and `abc` are treated the same, so just -1 to act
            // like the newline doesn't exist
            1 => -1,
            // this only happens if file is empty, just ignore
            0 => 0,
            // `abc\n\n` is trimmed to `abc`
            _ => 0,
        }
        + match suppress_newline {
            true => 1,
            false => 0, // compensate for the `\n` that is printed above
        };
    //
    Ok(bytes_saved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::read_to_string;

    fn test_data() -> Vec<(&'static str, &'static str, i32)> {
        vec![
            // empty
            ("", "", 1),
            // nothing to trim
            ("abc", "abc", 1),
            ("\nasd fgh\nabc", "\nasd fgh\nabc", 1),
            // `\r` is not used as a line break
            ("ab \t  \r abc", "ab \t  \r abc", 1),
            // `\r` is a whitespace though
            ("ab \t  \r \nabc", "ab\nabc", 7),
            ("\n \n \n\t\t \t \n \t\r \n\r   \r\r \n     \n \n", "", 33),
            // trailing newlines are removed
            ("ab \ncd \n  \n\n  \n", "ab\ncd", 11),
            // leading newlines are preserved
            ("  \n\t\r \r \n 123 absoi", "\n\n 123 absoi", 8),
            // weirdness with `std::io::BufReader`
            // this is shown in the tests for `crate::util::tests::readlines` but
            // 1. bufreader sees no difference between `[^\n]+\n` and ``
            ("abc", "abc", 1),
            ("abc\n", "abc", 1),
            ("abc\n\n", "abc", 3),
            ("abc ", "abc", 2),
            ("abc \n", "abc", 2),
        ]
    }

    mod stdout {
        use super::*;

        /// trim and output to stdout, with default options
        #[test]
        fn parametrized_n_suppress_newline() {
            test_data().into_par_iter().enumerate().for_each(
                |(index, (input, expected_raw, savings))| {
                    // make a "uniq" file
                    let prefix = format!("{}_{}_{}_{}", module_path!(), line!(), column!(), index);
                    let path_to_temp = mktemp(&prefix, &input).unwrap();

                    // make sure that its content is written correctly
                    let content = read_to_string(&path_to_temp).unwrap();
                    assert_eq!(input, content);

                    //
                    let mut result = Vec::new();
                    let lines = readlines(&path_to_temp).unwrap();
                    let tr = trim_custom(lines, &mut result, &mut None::<File>, false).unwrap();
                    //
                    let expected = format!("{}\n", expected_raw);

                    assert_eq!(savings - 1, tr); // `- 1` because not suppressing `\n`
                    assert_eq!(expected.as_bytes(), &result[..]);
                },
            );
        }

        /// trim and output to stdout, suppressing the trailing newline in the last line
        #[test]
        fn parametrized_suppress_newline() {
            test_data().into_par_iter().enumerate().for_each(
                |(index, (input, expected_raw, savings))| {
                    // make a "uniq" file
                    let prefix = format!("{}_{}_{}_{}", module_path!(), line!(), column!(), index);
                    let path_to_temp = mktemp(&prefix, &input).unwrap();

                    // make sure that its content is written correctly
                    let content = read_to_string(&path_to_temp).unwrap();
                    assert_eq!(input, content);

                    //
                    let mut result = Vec::new();
                    let lines = readlines(&path_to_temp).unwrap();
                    let tr = trim_custom(lines, &mut result, &mut None::<File>, true).unwrap();
                    //
                    let expected = format!("{}", expected_raw);

                    assert_eq!(savings, tr);
                    assert_eq!(expected.as_bytes(), &result[..]);
                },
            );
        }
    }

    mod in_place_sequential {
        use super::*;

        /// trim a file in place, with default options
        #[test]
        fn parametrized_inplace_n_suppress_newline() {
            test_data().into_par_iter().enumerate().for_each(
                |(index, (input, expected_raw, savings))| {
                    // make a "uniq" file
                    let prefix = format!("{}_{}_{}_{}", module_path!(), line!(), column!(), index);
                    let path_to_temp = mktemp(&prefix, &input).unwrap();

                    // make sure that its content is written correctly
                    let content = read_to_string(&path_to_temp).unwrap();
                    assert_eq!(input, content);

                    // trim the file in-place, sequentially
                    trim_files(&vec![path_to_temp.clone()], false)
                        .into_par_iter()
                        .for_each(|(file_opt, trim_result_res)| {
                            assert!(file_opt.exists());
                            match trim_result_res {
                                // `- 1` because not suppressing `\n`
                                Ok(tr) => assert_eq!(savings - 1, tr.bytes_saved),
                                _ => panic!(),
                            };
                        });

                    let expected = format!("{}\n", expected_raw);
                    let result = read_to_string(&path_to_temp).unwrap();
                    assert_eq!(expected, result);
                },
            );
        }

        /// trim a file in place, suppressing the trailing newline in the last line
        #[test]
        fn parametrized_inplace_suppress_newline() {
            test_data().into_par_iter().enumerate().for_each(
                |(index, (input, expected_raw, savings))| {
                    // make a "uniq" file
                    let prefix = format!("{}_{}_{}_{}", module_path!(), line!(), column!(), index);
                    let path_to_temp = mktemp(&prefix, &input).unwrap();

                    // make sure that its content is written correctly
                    let content = read_to_string(&path_to_temp).unwrap();
                    assert_eq!(input, content);

                    // trim the file in-place, sequentially
                    trim_files(&vec![path_to_temp.clone()], true)
                        .into_par_iter()
                        .for_each(|(file_opt, trim_result_res)| {
                            assert!(file_opt.exists());
                            match trim_result_res {
                                Ok(tr) => assert_eq!(savings, tr.bytes_saved),
                                _ => panic!(),
                            };
                        });

                    let expected = format!("{}", expected_raw);
                    let result = read_to_string(&path_to_temp).unwrap();
                    assert_eq!(expected, result);
                },
            );
        }
    }

    mod in_place_parallel {
        use super::*;

        /// trim a file in place, with default options
        #[test]
        fn parametrized_inplace_n_suppress_newline() {
            // path to file containing untrimmed content -> expected trimmed content
            let path_to_expected: HashMap<_, _> = test_data()
                .into_par_iter()
                .enumerate()
                .map(|(index, (input, expected_raw, savings))| {
                    // make a "uniq" file
                    let prefix = format!("{}_{}_{}_{}", module_path!(), line!(), column!(), index);
                    let path_to_temp = mktemp(&prefix, &input).unwrap();

                    // make sure that its content is written correctly
                    let content = read_to_string(&path_to_temp).unwrap();
                    assert_eq!(input, content);

                    //
                    let expected = format!("{}\n", expected_raw);
                    (path_to_temp, (expected, savings))
                })
                .collect();

            // collect all the paths and trim them all in one go
            let paths: Vec<_> = path_to_expected.keys().cloned().collect();
            let path_to_result: HashMap<_, _> = trim_files(&paths, false);

            // check the results
            path_to_expected
                .into_par_iter()
                .for_each(|(path_to_temp, (expected, savings))| {
                    let trim_result = path_to_result.get(&path_to_temp).unwrap().as_ref().unwrap();
                    let result = read_to_string(&path_to_temp).unwrap();
                    assert!(path_to_temp.exists());
                    // `- 1` because not suppressing `\n`
                    assert_eq!(savings - 1, trim_result.bytes_saved);
                    assert_eq!(expected, result);
                });
        }

        /// trim a file in place, suppressing the trailing newline in the last line
        #[test]
        fn parametrized_inplace_suppress_newline() {
            // path to file containing untrimmed content -> expected trimmed content
            let path_to_expected: HashMap<_, _> = test_data()
                .into_par_iter()
                .enumerate()
                .map(|(index, (input, expected_raw, savings))| {
                    // make a "uniq" file
                    let prefix = format!("{}_{}_{}_{}", module_path!(), line!(), column!(), index);
                    let path_to_temp = mktemp(&prefix, &input).unwrap();

                    // make sure that its content is written correctly
                    let content = read_to_string(&path_to_temp).unwrap();
                    assert_eq!(input, content);

                    //
                    let expected = format!("{}", expected_raw);
                    (path_to_temp, (expected, savings))
                })
                .collect();

            // collect all the paths and trim them all in one go
            let paths: Vec<_> = path_to_expected.keys().cloned().collect();
            let path_to_result: HashMap<_, _> = trim_files(&paths, true);

            // check the results
            path_to_expected
                .into_par_iter()
                .for_each(|(path_to_temp, (expected, savings))| {
                    let trim_result = path_to_result.get(&path_to_temp).unwrap().as_ref().unwrap();
                    let result = read_to_string(&path_to_temp).unwrap();
                    assert!(path_to_temp.exists());
                    assert_eq!(savings, trim_result.bytes_saved);
                    assert_eq!(expected, result);
                });
        }
    }
}
