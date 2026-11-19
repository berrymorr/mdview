mod cli;
mod code;
mod generate;
mod html;
mod output;
mod render;
mod style;
mod table;
mod text;
mod theme;

use std::io::{self, IsTerminal, Read};
use std::path::{Path, PathBuf};

use cli::When;
use text::Line;
use theme::Theme;

fn main() {
    let code = match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("mdview: {e}");
            1
        }
    };
    std::process::exit(code);
}

fn run() -> Result<i32, String> {
    let args = cli::parse();

    if let Some(what) = args.generate {
        generate::print(what)?;
        return Ok(0);
    }

    if args.list_themes {
        let mut list = String::new();
        for name in code::theme_names() {
            list.push_str(&name);
            list.push('\n');
        }
        output::emit(&list, false, false, None)?;
        return Ok(0);
    }

    let stdout_is_tty = io::stdout().is_terminal();
    let color = match args.color {
        When::Always => true,
        When::Never => false,
        When::Auto => {
            stdout_is_tty
                && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
                && std::env::var("TERM").map_or(true, |t| t != "dumb")
        }
    };
    let paging = match (args.no_paging, args.paging) {
        (true, _) | (_, When::Never) => false,
        (_, When::Always) => true,
        (_, When::Auto) => stdout_is_tty,
    };

    let true_color = std::env::var("COLORTERM").is_ok_and(|v| v == "truecolor" || v == "24bit");
    let mut theme = Theme::default();
    for &(name, style) in &args.colors {
        theme.set(name, style);
    }
    if args.italic {
        theme.emph = theme.emph.italic();
    }

    if args.list_colors {
        let lines = theme
            .styles()
            .into_iter()
            .map(|(name, style)| Line::plain(format!("{name}={style}"), fit(*style, true_color)))
            .collect();
        let mut list = String::new();
        text::write_lines(lines, color, &mut list);
        output::emit(&list, false, false, None)?;
        return Ok(0);
    }

    for (_, style) in theme.styles() {
        *style = fit(*style, true_color);
    }
    let width = args.width.map_or_else(terminal_width, usize::from);
    let settings = render::Settings {
        width,
        hyperlinks: color && args.hyperlinks,
        italic: args.italic,
        true_color,
        code_theme: code_theme(args.theme.as_deref()),
        theme,
    };

    let inputs: Vec<Option<PathBuf>> = if args.files.is_empty() {
        vec![None]
    } else {
        args.files
            .iter()
            .map(|f| (f != Path::new("-")).then(|| f.clone()))
            .collect()
    };
    let several = inputs.len() > 1;

    let mut text = String::new();
    let mut failed = false;
    let mut shown = 0;
    for input in &inputs {
        let name = input
            .as_ref()
            .map_or_else(|| "STDIN".to_string(), |p| p.display().to_string());
        let markdown = match read(input.as_deref()) {
            Ok(markdown) => markdown,
            Err(e) => {
                eprintln!("mdview: {name}: {e}");
                failed = true;
                continue;
            }
        };
        let mut lines = render::render(&markdown, &settings);
        if several {
            let style = settings.theme.file_header;
            let mut header = Line::plain(format!(" {name} "), style);
            header.pad_to(width, style);
            let mut block = vec![header, Line::new()];
            if shown > 0 {
                block.insert(0, Line::new());
            }
            lines.splice(0..0, block);
        }
        text::write_lines(lines, color, &mut text);
        shown += 1;
    }

    output::emit(
        &text,
        paging && !text.is_empty(),
        args.paging == When::Auto,
        args.pager.as_deref(),
    )?;
    Ok(if failed { 1 } else { 0 })
}

/// The style as the terminal can draw it: 24-bit colours only where
/// COLORTERM says the terminal takes them, as bat decides it for code.
fn fit(style: style::Style, true_color: bool) -> style::Style {
    if true_color {
        style
    } else {
        style.to_256_colors()
    }
}

/// The file's text, or standard input's for `None`: UTF-8 (invalid bytes
/// replaced), without a byte order mark, with Unix line endings.
fn read(path: Option<&Path>) -> io::Result<String> {
    let bytes = match path {
        Some(path) => std::fs::read(path)?,
        None => {
            let mut bytes = Vec::new();
            io::stdin().read_to_end(&mut bytes)?;
            bytes
        }
    };
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    Ok(text.replace("\r\n", "\n"))
}

/// The terminal's width: of standard output, or of standard error when the
/// output is piped; then COLUMNS; then 80.
fn terminal_width() -> usize {
    let size = terminal_size::terminal_size_of(io::stdout())
        .or_else(|| terminal_size::terminal_size_of(io::stderr()));
    if let Some((terminal_size::Width(w), _)) = size {
        return usize::from(w).max(10);
    }
    std::env::var("COLUMNS")
        .ok()
        .and_then(|c| c.parse().ok())
        .filter(|&w: &usize| w >= 10)
        .unwrap_or(80)
}

/// The syntax theme for code blocks. A name given on the command line must
/// exist; one from the environment may be one of bat's custom themes, which
/// mdview does not have, so that falls back quietly.
fn code_theme(flag: Option<&str>) -> String {
    if let Some(name) = flag {
        if code::theme_exists(name) {
            return name.to_string();
        }
        eprintln!(
            "mdview: unknown theme '{name}', using '{}' (see --list-themes)",
            code::DEFAULT_THEME
        );
        return code::DEFAULT_THEME.to_string();
    }
    ["MDVIEW_THEME", "BAT_THEME"]
        .iter()
        .filter_map(|var| std::env::var(var).ok())
        .find(|name| code::theme_exists(name))
        .unwrap_or_else(|| code::DEFAULT_THEME.to_string())
}
