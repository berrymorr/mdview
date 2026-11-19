//! Command-line arguments, with defaults from a configuration file in the
//! manner of bat's: one argument per line, `#` starts a comment.

use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use crate::generate::Generate;
use crate::style::Style;
use crate::theme::Theme;

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
                  Configuration: ~/.config/mdview/config holds default arguments, one per line \
                  (--colors among them).",
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

    /// The look of one part of the document, as ELEMENT=STYLE: 'emph=cyan',
    /// 'h1=bold brightred', 'mark=black yellow'. STYLE is written as git writes
    /// colours: attributes (bold, dim, italic, ul, reverse, strike) and up to two
    /// colours, the text's and the background's. Repeat it for more elements;
    /// --list-colors shows them all.
    #[arg(long, value_name = "ELEMENT=STYLE", value_parser = color_setting)]
    pub colors: Vec<(&'static str, Style)>,

    /// List the elements --colors can change, each in its current style.
    #[arg(long)]
    pub list_colors: bool,

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

    #[arg(long, value_enum, value_name = "WHAT", hide = true)]
    pub generate: Option<Generate>,
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

/// Reads one `--colors` value.
fn color_setting(value: &str) -> Result<(&'static str, Style), String> {
    let (name, spec) = value
        .split_once('=')
        .ok_or("expected ELEMENT=STYLE, such as 'emph=cyan'")?;
    let name = name.trim().to_ascii_lowercase();
    let Some(name) = Theme::names().into_iter().find(|known| *known == name) else {
        return Err(format!(
            "there is no element '{name}' (--list-colors shows them all)"
        ));
    };
    Ok((name, Style::parse(spec)?))
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
    fn colors_add_up_in_order() {
        let args = Args::parse_from([
            "mdview",
            "--colors=emph=cyan",
            "--colors",
            "H1=bold red",
            "--colors=emph=magenta",
        ]);
        let names: Vec<&str> = args.colors.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, ["emph", "h1", "emph"]);
        assert_eq!(args.colors[2].1, Style::parse("magenta").unwrap());
    }

    #[test]
    fn colors_name_a_known_element() {
        assert!(color_setting("emph").is_err());
        assert!(color_setting("h7=red").is_err());
        assert!(color_setting("emph=blod").is_err());
        assert_eq!(
            color_setting("code-frame=").unwrap(),
            ("code-frame", Style::PLAIN)
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
