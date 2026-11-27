//! Command-line argument parsing and validation for the Ned editor binary.

use std::path::PathBuf;

/// Parsed command line arguments.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct cli_args {
    /// Whether to display help text and exit.
    pub help: bool,
    /// Whether to display version text and exit.
    pub version: bool,
    /// List of file paths to open into editor buffers.
    pub files: Vec<PathBuf>,
}

/// Command line parsing error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum cli_error {
    /// An unknown option flag was supplied.
    unknown_option(String),
}

impl std::fmt::Display for cli_error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::unknown_option(opt) => write!(f, "error: unexpected argument '{opt}' found"),
        }
    }
}

impl std::error::Error for cli_error {}

impl cli_args {
    /// Parses an iterator of command-line arguments (excluding program name).
    ///
    /// # Errors
    ///
    /// Returns `cli_error::unknown_option` if an unrecognized option flag is encountered before `--`.
    pub fn parse<S: AsRef<str>, I: IntoIterator<Item = S>>(args: I) -> Result<Self, cli_error> {
        let mut parsed = Self::default();
        let mut iter = args.into_iter();

        while let Some(arg) = iter.next() {
            let s = arg.as_ref();
            if s == "--" {
                // All remaining tokens are treated as file paths, even if they begin with '-'
                for rem in iter {
                    parsed.files.push(PathBuf::from(rem.as_ref()));
                }
                break;
            } else if s == "-h" || s == "--help" {
                parsed.help = true;
            } else if s == "-v" || s == "--version" {
                parsed.version = true;
            } else if s.starts_with('-') && s != "-" {
                return Err(cli_error::unknown_option(s.to_string()));
            } else {
                parsed.files.push(PathBuf::from(s));
            }
        }

        Ok(parsed)
    }

    /// Returns standard help documentation text.
    #[must_use]
    pub fn help_text() -> &'static str {
        concat!(
            "ned: fast, disciplined modal terminal text editor\n\n",
            "Usage: ned [OPTIONS] [FILES]...\n\n",
            "Arguments:\n",
            "  [FILES]...  Files to open in editor buffers\n\n",
            "Options:\n",
            "  -h, --help     Print help information\n",
            "  -v, --version  Print version information\n"
        )
    }

    /// Returns version text.
    #[must_use]
    pub fn version_text() -> String {
        format!("ned {}", env!("CARGO_PKG_VERSION"))
    }
}
