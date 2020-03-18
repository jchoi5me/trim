use ansi_term::Colour::Green;
use ansi_term::Colour::Red;
use ansi_term::Colour::White;
use ansi_term::Style;
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fmt::Display;
use std::fs::remove_file;
use std::fs::File;
use std::hash::Hash;
use std::hash::Hasher;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

/// # Returns
///
/// Hash of `hashable` obtained using `std::collections::hash_map::DefaultHasher`.
pub fn hash_default<H>(hashable: &H) -> u64
where
    H: Hash,
{
    let mut hasher = DefaultHasher::new();
    hashable.hash(&mut hasher);
    hasher.finish()
}

/// # Returns
///
/// Path to some newly created tempfile in some OS-managed tempdir. The basename of this file
/// will be prefixed with `prefix` and its content equal to `content`.
pub fn mktemp<D>(prefix: &str, content: &D) -> io::Result<PathBuf>
where
    D: Display,
{
    let basename = PathBuf::from(prefix);
    let path_to_temp = env::temp_dir().as_path().join(&basename);
    if path_to_temp.exists() {
        remove_file(&path_to_temp)?;
    }

    let mut file = File::create(&path_to_temp)?;
    write!(&mut file, "{}", content).unwrap(); // write untrimmed content
    Ok(path_to_temp)
}

/// # Returns
///
/// An `Iterator` that reads through the file under `path` line by line, delimited by `\n` or `\r\n`.
#[inline]
pub fn readlines(path: &Path) -> io::Result<impl Iterator<Item = io::Result<String>>> {
    File::open(path).map(BufReader::new).map(BufReader::lines)
}

/// Used to visualize the trimmed whitespace.
///
/// # Returns
///
/// Some `impl Display` that results in `length` contiguous chars with a red background and a
/// white foreground, each with `_` as the text.
#[inline]
pub fn red_padding_with_len(length: usize) -> impl Display {
    Style::new()
        .on(Red)
        .fg(White)
        .paint((0..length).map(|_| '_').collect::<String>())
}

/// # Returns
///
/// Some `impl Display` that results in the original `text` with green font.
#[inline]
pub fn red(text: &str) -> impl Display {
    Style::new().fg(Red).paint(String::from(text))
}

/// # Returns
///
/// Some `impl Display` that results in the original `text` with red font.
#[inline]
pub fn green(text: &str) -> impl Display {
    Style::new().fg(Green).paint(String::from(text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rayon::prelude::*;

    mod readlines {
        use super::*;
        fn test_data() -> Vec<(&'static str, Vec<&'static str>)> {
            vec![
                ("abc", vec!["abc"]),
                ("abc\n", vec!["abc"]),
                ("abc\n\n", vec!["abc", ""]),
                ("\nabc", vec!["", "abc"]),
                ("\n\nabc", vec!["", "", "abc"]),
                // CRLF instead of LF
                ("abc", vec!["abc"]),
                ("abc\r\n", vec!["abc"]),
                ("abc\r\n\r\n", vec!["abc", ""]),
                ("\r\nabc", vec!["", "abc"]),
                ("\r\n\r\nabc", vec!["", "", "abc"]),
            ]
        }

        #[test]
        fn parametrized_readlines() {
            test_data()
                .into_par_iter()
                .enumerate()
                .for_each(|(index, (input, expected))| {
                    let prefix = format!("{}_{}_{}_{}", module_path!(), line!(), column!(), index);
                    let path_to_temp = mktemp(&prefix, &input).unwrap();
                    let result_lines: Vec<_> = readlines(&path_to_temp)
                        .unwrap()
                        .into_iter()
                        .map(Result::unwrap)
                        .collect();
                    let expected_lines: Vec<_> = expected.into_iter().map(String::from).collect();
                    assert_eq!(expected_lines, result_lines);
                });
        }
    }
}
