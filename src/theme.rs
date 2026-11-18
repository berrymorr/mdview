//! How each piece of Markdown looks: colours, attributes and the characters
//! drawn for structure.
//!
//! Everything uses the 16 palette colours, so the terminal's own colour
//! scheme decides the exact shades. Italic is a colour here rather than the
//! SGR italic attribute, which many terminals cannot show; `--italic` adds
//! the attribute on top. `--colors` changes any of the styles by name.

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

impl Default for Theme {
    fn default() -> Theme {
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
            emph: Style::fg(GREEN),
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

impl Theme {
    /// Every style `--colors` can set, under the name it is set by.
    pub fn styles(&mut self) -> Vec<(&'static str, &mut Style)> {
        let Theme {
            headings,
            strong,
            emph,
            strike,
            code,
            mark,
            kbd,
            link,
            url,
            image,
            footnote,
            math,
            html,
            quote_bar,
            quote_text,
            list_marker,
            task_open,
            task_done,
            rule,
            table_border,
            table_header,
            code_frame,
            code_label,
            code_text,
            term,
            file_header,
            alerts: [note, tip, important, warning, caution],
        } = self;
        const HEADINGS: [(&str, &str); 6] = [
            ("h1", "h1-rule"),
            ("h2", "h2-rule"),
            ("h3", "h3-rule"),
            ("h4", "h4-rule"),
            ("h5", "h5-rule"),
            ("h6", "h6-rule"),
        ];
        let mut styles = Vec::new();
        for (heading, (name, rule_name)) in headings.iter_mut().zip(HEADINGS) {
            styles.push((name, &mut heading.text));
            if let Some((_, style)) = &mut heading.rule {
                styles.push((rule_name, style));
            }
        }
        styles.extend([
            ("strong", strong),
            ("emph", emph),
            ("strike", strike),
            ("code", code),
            ("mark", mark),
            ("kbd", kbd),
            ("link", link),
            ("url", url),
            ("image", image),
            ("footnote", footnote),
            ("math", math),
            ("html", html),
            ("quote-bar", quote_bar),
            ("quote-text", quote_text),
            ("list-marker", list_marker),
            ("task-open", task_open),
            ("task-done", task_done),
            ("rule", rule),
            ("table-border", table_border),
            ("table-header", table_header),
            ("code-frame", code_frame),
            ("code-label", code_label),
            ("code-text", code_text),
            ("term", term),
            ("file-header", file_header),
            ("note", note),
            ("tip", tip),
            ("important", important),
            ("warning", warning),
            ("caution", caution),
        ]);
        styles
    }

    /// The names `--colors` knows.
    pub fn names() -> Vec<&'static str> {
        Theme::default()
            .styles()
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    }

    /// Gives the style called `name` a new look; false if there is none by
    /// that name.
    pub fn set(&mut self, name: &str, style: Style) -> bool {
        match self.styles().into_iter().find(|(known, _)| *known == name) {
            Some((_, slot)) => {
                *slot = style;
                true
            }
            None => false,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_style_has_its_own_name() {
        let names = Theme::names();
        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), names.len());
        assert!(names.contains(&"h1-rule") && !names.contains(&"h3-rule"));
    }

    #[test]
    fn a_style_is_set_by_name() {
        let mut theme = Theme::default();
        assert!(theme.set("caution", Style::fg(GREEN)));
        assert!(theme.set("h2-rule", Style::PLAIN.dim()));
        assert_eq!(theme.alerts[4], Style::fg(GREEN));
        assert_eq!(theme.headings[1].rule, Some(('─', Style::PLAIN.dim())));
        assert!(!theme.set("h7", Style::PLAIN));
    }
}
