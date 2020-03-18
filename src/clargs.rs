use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "trim")]
pub struct Opt {
    /// trim <files> in-place, overwritting the content of the files atomically
    #[structopt(short = "i", long = "in-place")]
    pub in_place: bool,

    /// suppress outputting the trailing `\n` in the last line
    #[structopt(short = "N", long = "supress-newline")]
    pub suppress_newline: bool,

    /// suppress summary
    #[structopt(short = "S", long = "supress-summary")]
    pub suppress_summary: bool,

    /// suppress visualizations of the trim
    #[structopt(short = "V", long = "supress-visual")]
    pub suppress_visual: bool,

    /// files to trim; if '-' exists or none provided, stdin will be used
    #[structopt(parse(from_os_str))]
    pub files: Vec<PathBuf>,
}
