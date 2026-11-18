//! Command-line arguments, with defaults from a configuration file in the
//! manner of bat's: one argument per line, `#` starts a comment.

use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum When {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Parser)]
#[command(
    name = "mdview",
    version,
    about = "Show Markdown files in the terminal, rendered: headings, emphasis, lists, \
             tables and highlighted code instead of the raw markup.",
    after_help = "Reads standard input when no FILE is given or FILE is '-'.\n\n\
                  Environment: MDVIEW_PAGER or PAGER (the pager), MDVIEW_THEME or BAT_THEME \
                  (theme for code blocks), NO_COLOR, MDVIEW_CONFIG_PATH.\n\
                  Configuration: ~/.config/mdview/config holds default arguments, one per line.",
    args_override_self = true
)]
pub struct Args {
    /// Files to show.
    #[arg(value_name = "FILE")]
    pub files: Vec<PathBuf>,

    /// When to use a pager. 'auto' pages when the output is a terminal, and the
    /// pager quits by itself if the text fits on one screen.
    #[arg(long, value_enum, value_name = "WHEN", default_value_t = When::Auto)]
    pub paging: When,

    /// Never use a pager (the same as --paging=never).
    #[arg(short = 'P', long)]
    pub no_paging: bool,

    /// Pager command, such as "less -R". Takes precedence over MDVIEW_PAGER and
    /// PAGER; an empty command means no pager.
    #[arg(long, value_name = "COMMAND")]
    pub pager: Option<String>,

    /// When to use colours and text attributes. 'auto' uses them when the output
    /// is a terminal and NO_COLOR is not set.
    #[arg(long, value_enum, value_name = "WHEN", default_value_t = When::Auto)]
    pub color: When,

    /// Width to lay the text out in, in columns [default: the terminal's width].
    #[arg(short, long, value_name = "COLUMNS", value_parser = clap::value_parser!(u16).range(10..))]
    pub width: Option<u16>,

    /// Syntax theme for code blocks; --list-themes shows the choice.
    /// [default: MDVIEW_THEME, then BAT_THEME, then "Monokai Extended"]
    #[arg(long, value_name = "NAME")]
    pub theme: Option<String>,

    /// List the syntax themes for code blocks.
    #[arg(long)]
    pub list_themes: bool,

    /// Make links clickable (OSC 8 hyperlinks), for terminals that support them.
    #[arg(long, overrides_with = "no_hyperlinks")]
    pub hyperlinks: bool,

    #[arg(long, overrides_with = "hyperlinks", hide = true)]
    pub no_hyperlinks: bool,

    /// Draw emphasis in italics as well as in its colour, for terminals that
    /// can show italic text.
    #[arg(long, overrides_with = "no_italic")]
    pub italic: bool,

    #[arg(long, overrides_with = "italic", hide = true)]
    pub no_italic: bool,

    /// Do not read the configuration file.
    #[arg(long)]
    pub no_config: bool,
}

/// Parses the command line, after the configuration file's arguments so
/// that the command line wins.
pub fn parse() -> Args {
    let mut argv: Vec<OsString> = std::env::args_os().collect();
    let given = argv.split_off(1.min(argv.len()));
    if !given.iter().any(|a| a == "--no-config")
        && let Some(path) = config_path()
        && let Ok(text) = std::fs::read_to_string(&path)
    {
        match config_args(&text) {
            Ok(args) => argv.extend(args.into_iter().map(OsString::from)),
            Err(e) => eprintln!("mdview: {}: {e}", path.display()),
        }
    }
    argv.extend(given);
    Args::parse_from(argv)
}

fn config_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("MDVIEW_CONFIG_PATH").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(path));
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("mdview").join("config"))
}

fn config_args(text: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let words = shell_words::split(line).map_err(|e| format!("line {}: {e}", n + 1))?;
        args.extend(words);
    }
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_lines_become_arguments() {
        let text = "# defaults\n--theme=\"Monokai Extended Light\"\n\n  --width 100\n--italic # trailing\n";
        let args = config_args(text).unwrap();
        assert_eq!(
            args,
            [
                "--theme=Monokai Extended Light",
                "--width",
                "100",
                "--italic"
            ]
        );
    }

    #[test]
    fn later_arguments_override_earlier_ones() {
        let args = Args::parse_from([
            "mdview",
            "--width",
            "100",
            "--italic",
            "-w",
            "60",
            "--no-italic",
        ]);
        assert_eq!(args.width, Some(60));
        assert!(!args.italic && args.no_italic);
    }
}
