use colmac::*;
use std::cmp::min;
use std::io;
use std::io::stdin;
use std::io::BufRead;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;
use structopt::StructOpt;

mod clargs;
mod trim;
mod util;

use crate::clargs::Opt;
use crate::trim::*;
use crate::util::*;

fn main() {
    // cli args
    let Opt {
        files,
        in_place,
        suppress_newline,
        suppress_summary,
        suppress_visual,
    } = Opt::from_args();

    let no_files_provided = files.len() == 0;
    let dash_provided = files
        .iter()
        .map(PathBuf::as_path)
        .map(&Path::to_str)
        .any(|x| x == Some("-"));
    let use_stdin = no_files_provided || dash_provided;

    // switch on some of the cli options
    // if key is `None`, this implies that stdin was used
    let summaries: HashMap<Option<PathBuf>, io::Result<TrimResult>> = match in_place {
        // ERROR: cannot do in-place edit using stdin
        true if use_stdin => panic!("Cannot read from stdin if `-i` is specified"),
        // in-place trim every file
        true => {
            eprintln!("Trimming {} files in-place...\n", files.len());
            trim_files(&files, suppress_newline)
                .into_iter()
                .map(|(path_buf, trim_result)| (Some(path_buf), trim_result))
                .collect()
        }
        // trim lines from stdin
        false if use_stdin => {
            // nonessential; just report what's happening
            eprintln!(
                "{}; reading lines from stdin...",
                match no_files_provided {
                    // okay if no files are provided; just read from stdin
                    true => "No files provided",
                    // okay if `-` is the only arg provided
                    false if dash_provided && files.len() == 1 => "`-` provided",
                    // not okay if `-` is provided along with other file names
                    false if dash_provided => panic!("Can't mix `-` with other files"),
                    false => unreachable!(),
                }
            );

            hashmap![
                None => trim_iter(stdin().lock().lines(), suppress_visual, suppress_newline)
            ]
        }
        // trim lines from a file to stdout; ensuring that only one file is provided
        false => match files.get(0) {
            Some(path) if files.len() == 1 => {
                eprintln!("Reading lines from {:?}...", path);
                let filename = Some(PathBuf::from(path));
                let result = match readlines(&path) {
                    Ok(lines) => trim_iter(lines, suppress_visual, suppress_newline),
                    Err(err) => Err(err),
                };
                hashmap![ filename => result ]
            }
            _ => panic!("Cannot handle multiple files without `-i`"),
        },
    };

    // newline to separate summary from visual
    if !suppress_summary {
        eprint!("\n");
    }
    // sum up all the exit codes, so if it's > 0, at least one error occurred
    let exit_code_sum: i32 = summaries
        .into_iter()
        .map(|(file_opt, summary_res)| {
            let filename = match file_opt {
                Some(file) => format!("{:?}", file),
                None => format!("stdin"),
            };
            (filename, summary_res)
        })
        .map(|(filename, summary_res)| match summary_res {
            Ok(TrimResult { bytes_saved }) if !suppress_summary => {
                // color the filename green if bytes were saved, don't otherwise
                let filename_colored = match bytes_saved {
                    0 => format!("{}", &filename),
                    _ => format!("{}", green(&filename)),
                };
                eprintln!("{:>6} bytes ish from {}", bytes_saved, filename_colored);
                0
            }
            Err(err) => {
                eprintln!("ERROR with {}: {}", red(&filename), err);
                1
            }
            _ => 0,
        })
        .sum();

    // truncate for consistency
    let exit_code = min(1, exit_code_sum);
    exit(exit_code);
}
