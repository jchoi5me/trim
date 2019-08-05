# trim

- [crates.io](https://crates.io/crates/trim)

`trim` was inspired by the auto-trimming feature of [Atom](https://atom.io/), where trailing whitespace of every line is removed, and trailing newlines are replaced with a single newline.

```bash
~
  $ trim --help
trim 0.1.5
Jack <jchoi5@me.com>
trim whitespaces from files

USAGE:
    trim [files]...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <files>...    Files to trim. If empty or is '-', stdin will be used to grab lines.
```
