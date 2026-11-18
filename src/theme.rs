//! How each piece of Markdown looks: colours, attributes and the characters
//! drawn for structure.
//!
//! Everything uses the 16 palette colours, so the terminal's own colour
//! scheme decides the exact shades. Italic is a colour here rather than the
//! SGR italic attribute, which many terminals cannot show; `--italic` adds
//! the attribute on top.

use crate::style::*;

pub struct Heading {
    pub text: Style,
    /// A line drawn under the heading across the full width.
    pub rule: Option<(char, Style)>,
}

pub struct Theme {
    pub headings: [Heading; 6],
    pub strong: Style,
    pub emph: Style,
    pub strike: Style,
    pub code: Style,
    pub mark: Style,
    pub kbd: Style,
    pub link: Style,
    pub url: Style,
    pub image: Style,
    pub footnote: Style,
    pub math: Style,
    pub html: Style,
    pub quote_bar: Style,
    pub quote_text: Style,
    pub list_marker: Style,
    pub task_open: Style,
    pub task_done: Style,
    pub rule: Style,
    pub table_border: Style,
    pub table_header: Style,
    pub code_frame: Style,
    pub code_label: Style,
    pub code_text: Style,
    pub term: Style,
    pub file_header: Style,
    /// Note, Tip, Important, Warning, Caution.
    pub alerts: [Style; 5],
}

impl Theme {
    pub fn new(italic: bool) -> Theme {
        let emph = Style::fg(GREEN);
        Theme {
            headings: [
                Heading {
                    text: Style::fg(BRIGHT_MAGENTA).bold(),
                    rule: Some(('━', Style::fg(MAGENTA))),
                },
                Heading {
                    text: Style::fg(BRIGHT_BLUE).bold(),
                    rule: Some(('─', Style::fg(BLUE))),
                },
                Heading {
                    text: Style::fg(BRIGHT_CYAN).bold(),
                    rule: None,
                },
                Heading {
                    text: Style::fg(BRIGHT_GREEN).bold(),
                    rule: None,
                },
                Heading {
                    text: Style::fg(BRIGHT_YELLOW).bold(),
                    rule: None,
                },
                Heading {
                    text: Style::PLAIN.bold(),
                    rule: None,
                },
            ],
            strong: Style::PLAIN.bold(),
            emph: if italic { emph.italic() } else { emph },
            strike: Style::PLAIN.dim().strike(),
            code: Style::fg(YELLOW),
            mark: Style::fg(BLACK).on(YELLOW),
            kbd: Style::PLAIN.reverse(),
            link: Style::fg(BRIGHT_BLUE).underline(),
            url: Style::PLAIN.dim(),
            image: Style::fg(MAGENTA),
            footnote: Style::fg(CYAN),
            math: Style::fg(MAGENTA),
            html: Style::PLAIN.dim(),
            quote_bar: Style::PLAIN.dim(),
            quote_text: Style::PLAIN,
            list_marker: Style::fg(YELLOW),
            task_open: Style::PLAIN.dim(),
            task_done: Style::fg(GREEN),
            rule: Style::PLAIN.dim(),
            table_border: Style::PLAIN.dim(),
            table_header: Style::PLAIN.bold(),
            code_frame: Style::PLAIN.dim(),
            code_label: Style::PLAIN.dim(),
            code_text: Style::PLAIN,
            term: Style::PLAIN.bold(),
            file_header: Style::PLAIN.bold().reverse(),
            alerts: [
                Style::fg(BRIGHT_BLUE),
                Style::fg(BRIGHT_GREEN),
                Style::fg(BRIGHT_MAGENTA),
                Style::fg(BRIGHT_YELLOW),
                Style::fg(BRIGHT_RED),
            ],
        }
    }
}

/// Characters drawn for structure.
pub struct Glyphs {
    /// Bullets by nesting depth; deeper lists cycle through them again.
    pub bullets: [&'static str; 3],
    pub task_open: &'static str,
    pub task_done: &'static str,
    pub quote_bar: &'static str,
    pub alert_bar: &'static str,
    pub rule: char,
    pub footnote_rule: char,
    pub table: BoxChars,
    pub code: CodeFrame,
}

/// Box-drawing set for tables: corners, junctions, lines.
pub struct BoxChars {
    pub horizontal: char,
    pub vertical: char,
    pub top: [char; 3],
    pub middle: [char; 3],
    pub bottom: [char; 3],
}

pub struct CodeFrame {
    pub top: char,
    pub side: char,
    pub bottom: char,
    pub line: char,
}

pub const GLYPHS: Glyphs = Glyphs {
    bullets: ["•", "◦", "▪"],
    task_open: "☐",
    task_done: "☑",
    quote_bar: "│",
    alert_bar: "┃",
    rule: '─',
    footnote_rule: '─',
    table: BoxChars {
        horizontal: '─',
        vertical: '│',
        top: ['┌', '┬', '┐'],
        middle: ['├', '┼', '┤'],
        bottom: ['└', '┴', '┘'],
    },
    code: CodeFrame {
        top: '┌',
        side: '│',
        bottom: '└',
        line: '─',
    },
};
