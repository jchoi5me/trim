use std::io::{stdin, BufRead};
use std::path::PathBuf;
use structopt::StructOpt;

mod prelude;

use prelude::*;

#[derive(StructOpt, Debug)]
#[structopt(name = "trim")]
struct Opt {
    /// Files to trim. If empty or is '-', stdin will be used to grab lines.
    #[structopt(parse(from_os_str))]
    pub files: Vec<PathBuf>,
}

fn handle_file(file: &PathBuf) -> TrimResult {
    handle_iter(readlines(file))
}

fn handle_iter(lines: impl Iterator<Item = String>) -> TrimResult {
    clean(lines)
}

fn report(tr: TrimResult) {
    let original_len = tr.trimmed.len() + tr.saved_bytes;
    print!("{}", tr.trimmed);
    eprintln!(
        "{}\n{} -> {} ({:.2} %)",
        tr.visualized,
        original_len,
        tr.trimmed.len(),
        100.0 * (1.0 - tr.saved_bytes as f64 / original_len as f64)
    );
}

fn main() {
    let args = Opt::from_args();

    // if 0 files is provided or `-` is the only, then use stdin
    let use_stdin = match args.files.get(0) {
        None => true,
        Some(path) if path.to_str() == Some("-") => true,
        _ => false,
    };
    if use_stdin {
        let result = handle_iter(stdin().lock().lines().map(Result::unwrap));
        report(result);
    } else {
        args.files.iter().map(handle_file).for_each(report)
    };
}
