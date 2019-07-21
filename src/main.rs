use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;

mod prelude;

use prelude::*;

#[derive(StructOpt, Debug)]
#[structopt(name = "trim")]
struct Opt {
    /// Files to trim. If empty or is '-', stdin will be used instead.
    #[structopt(parse(from_os_str))]
    pub files: Vec<PathBuf>,
}

fn handle_file(file: &PathBuf) -> (Box<String>, Box<String>) {
    let output = clean(readlines(&file));

    // STDERR print how mant bytes were removed
    let old_file_size = fs::metadata(&file).unwrap().len() as f64;
    let new_file_size = output.len() as f64 + 1.0;

    let vis_output = Box::new(format!(
        "{}\n{:?}: {} -> {} ({:.2}%)",
        visualize(&file, &output),
        file,
        old_file_size,
        new_file_size,
        new_file_size / old_file_size * 100.0
    ));

    (output, vis_output)
}

fn main() {
    let opt = Opt::from_args();

    opt.files
        .par_iter()
        .map(handle_file)
        .for_each(|(result, vis_result)| {
            print!("{}", result);
            eprintln!("{}", vis_result);
        });
}
