//! Syntax highlighting for code blocks, with the syntax definitions and
//! themes that bat ships (through the two-face crate).

use syntect::easy::HighlightLines;
use syntect::highlighting::{Color as SyntectColor, FontStyle, Style as SyntectStyle, Theme};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;
use two_face::theme::LazyThemeSet;

use crate::style::{Color, Style};
use crate::text::Span;

/// bat's own default theme.
pub const DEFAULT_THEME: &str = "Monokai Extended";

pub fn theme_names() -> Vec<String> {
    let themes: LazyThemeSet = two_face::theme::extra().into();
    themes.theme_names().map(str::to_string).collect()
}

pub fn theme_exists(name: &str) -> bool {
    theme_names().iter().any(|n| n == name)
}

pub struct Highlighter {
    syntaxes: SyntaxSet,
    theme: Theme,
    true_color: bool,
    italic: bool,
}

impl Highlighter {
    pub fn new(theme_name: &str, true_color: bool, italic: bool) -> Highlighter {
        let themes: LazyThemeSet = two_face::theme::extra().into();
        let theme = themes
            .get(theme_name)
            .or_else(|| themes.get(DEFAULT_THEME))
            .expect("the default theme is built in")
            .clone();
        Highlighter {
            syntaxes: two_face::syntax::extra_newlines(),
            theme,
            true_color,
            italic,
        }
    }

    /// The syntax a code block's language name stands for: a file
    /// extension (`rs`, `py`) or a syntax name (`rust`, `Dockerfile`).
    pub fn find(&self, language: &str) -> Option<&SyntaxReference> {
        let language = language.to_ascii_lowercase();
        let language = match language.as_str() {
            "shell" | "console" | "shell-session" | "shellsession" | "terminal" | "sh-session" => {
                "bash"
            }
            "golang" => "go",
            "csharp" => "cs",
            "py3" | "python3" => "py",
            "jsonc" | "json5" => "json",
            "text" | "plain" | "plaintext" | "txt" => return None,
            other => other,
        };
        self.syntaxes.find_syntax_by_token(language)
    }

    /// Highlights `code` as `syntax`: one run of spans per source line.
    pub fn highlight(&self, code: &str, syntax: &SyntaxReference) -> Vec<Vec<Span>> {
        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        LinesWithEndings::from(code)
            .map(
                |line| match highlighter.highlight_line(line, &self.syntaxes) {
                    Ok(regions) => regions
                        .into_iter()
                        .map(|(style, text)| {
                            Span::new(text.trim_end_matches('\n'), self.style(style))
                        })
                        .filter(|span| !span.text.is_empty())
                        .collect(),
                    Err(_) => vec![Span::new(line.trim_end_matches('\n'), Style::PLAIN)],
                },
            )
            .collect()
    }

    fn style(&self, style: SyntectStyle) -> Style {
        let mut out = Style::PLAIN;
        out.fg = self.color(style.foreground);
        out.bold = style.font_style.contains(FontStyle::BOLD);
        out.underline = style.font_style.contains(FontStyle::UNDERLINE);
        // Like bat's --italic-text=never by default: the terminal may not
        // be able to draw italics at all.
        out.italic = self.italic && style.font_style.contains(FontStyle::ITALIC);
        out
    }

    /// bat's colour conventions: themes such as `ansi` and `base16` put a
    /// palette index in the red channel with alpha 0, and alpha 1 means the
    /// terminal's default colour.
    fn color(&self, color: SyntectColor) -> Option<Color> {
        match color.a {
            0 if color.r < 16 => Some(Color::Ansi(color.r)),
            0 => Some(Color::Fixed(color.r)),
            1 => None,
            _ if self.true_color => Some(Color::Rgb(color.r, color.g, color.b)),
            _ => Some(Color::Fixed(ansi_colours::ansi256_from_rgb((
                color.r, color.g, color.b,
            )))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_by_name_and_extension() {
        let h = Highlighter::new(DEFAULT_THEME, true, false);
        assert_eq!(h.find("rust").unwrap().name, "Rust");
        assert_eq!(h.find("rs").unwrap().name, "Rust");
        assert_eq!(h.find("Python").unwrap().name, "Python");
        assert!(h.find("shell").unwrap().name.contains("bash"));
        assert!(h.find("text").is_none());
        assert!(h.find("no-such-language").is_none());
    }

    #[test]
    fn highlighting_colours_keywords() {
        let h = Highlighter::new(DEFAULT_THEME, true, false);
        let lines = h.highlight("fn main() {}\n", h.find("rust").unwrap());
        assert_eq!(lines.len(), 1);
        assert_eq!(crate::text::plain_text(&lines[0]), "fn main() {}");
        assert!(
            lines[0]
                .iter()
                .any(|s| s.text == "fn" && s.style.fg.is_some())
        );
    }

    #[test]
    fn ansi_theme_uses_the_palette() {
        let h = Highlighter::new("ansi", false, false);
        let lines = h.highlight("fn main() {}\n", h.find("rust").unwrap());
        assert!(
            lines[0]
                .iter()
                .all(|s| matches!(s.style.fg, None | Some(Color::Ansi(_))))
        );
    }
}
