//! Text attributes and colours, the SGR escape sequences they become, and
//! the words they are written in: git's way of writing colours in its
//! configuration, such as `bold brightred` or `black yellow`.

use std::fmt::{self, Write as _};

/// A colour as the terminal understands it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    /// One of the 16 palette colours: 0–7 normal, 8–15 bright. The terminal's
    /// own colour scheme decides what they look like, so they suit dark and
    /// light backgrounds alike and work on any terminal.
    Ansi(u8),
    /// An entry of the 256-colour palette.
    Fixed(u8),
    /// A 24-bit colour.
    Rgb(u8, u8, u8),
}

pub const BLACK: Color = Color::Ansi(0);
pub const GREEN: Color = Color::Ansi(2);
pub const YELLOW: Color = Color::Ansi(3);
pub const BLUE: Color = Color::Ansi(4);
pub const MAGENTA: Color = Color::Ansi(5);
pub const CYAN: Color = Color::Ansi(6);
pub const BRIGHT_RED: Color = Color::Ansi(9);
pub const BRIGHT_GREEN: Color = Color::Ansi(10);
pub const BRIGHT_YELLOW: Color = Color::Ansi(11);
pub const BRIGHT_BLUE: Color = Color::Ansi(12);
pub const BRIGHT_MAGENTA: Color = Color::Ansi(13);
pub const BRIGHT_CYAN: Color = Color::Ansi(14);

/// The eight palette colours by name; `bright` in front picks the other
/// eight.
const NAMES: [&str; 8] = [
    "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub reverse: bool,
}

impl Style {
    pub const PLAIN: Style = Style {
        fg: None,
        bg: None,
        bold: false,
        dim: false,
        italic: false,
        underline: false,
        strike: false,
        reverse: false,
    };

    pub const fn fg(color: Color) -> Style {
        Style {
            fg: Some(color),
            ..Style::PLAIN
        }
    }

    pub const fn on(mut self, color: Color) -> Style {
        self.bg = Some(color);
        self
    }

    pub const fn bold(mut self) -> Style {
        self.bold = true;
        self
    }

    pub const fn dim(mut self) -> Style {
        self.dim = true;
        self
    }

    pub const fn italic(mut self) -> Style {
        self.italic = true;
        self
    }

    pub const fn underline(mut self) -> Style {
        self.underline = true;
        self
    }

    pub const fn strike(mut self) -> Style {
        self.strike = true;
        self
    }

    pub const fn reverse(mut self) -> Style {
        self.reverse = true;
        self
    }

    /// `self` with `top` laid over it: a colour set in `top` wins, attributes
    /// add up. This is how nested markup combines — `**bold _and italic_**`.
    pub fn patch(self, top: Style) -> Style {
        Style {
            fg: top.fg.or(self.fg),
            bg: top.bg.or(self.bg),
            bold: self.bold || top.bold,
            dim: self.dim || top.dim,
            italic: self.italic || top.italic,
            underline: self.underline || top.underline,
            strike: self.strike || top.strike,
            reverse: self.reverse || top.reverse,
        }
    }

    /// Reads a style written as git writes colours: words in any order,
    /// separated by spaces or commas. The attributes are `bold`, `dim`,
    /// `italic`, `ul` (or `underline`), `reverse` and `strike`. The first
    /// colour is the text's and the second the background's; `normal` is the
    /// terminal's own, so `normal blue` sets only the background. A style with
    /// no words, or just `normal`, is plain text.
    pub fn parse(spec: &str) -> Result<Style, String> {
        let mut style = Style::PLAIN;
        let mut colors = 0;
        let words = spec
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|w| !w.is_empty());
        for word in words {
            let lower = word.to_ascii_lowercase();
            match lower.as_str() {
                "bold" => style.bold = true,
                "dim" => style.dim = true,
                "italic" => style.italic = true,
                "ul" | "underline" => style.underline = true,
                "reverse" => style.reverse = true,
                "strike" => style.strike = true,
                _ => {
                    let color = parse_color(&lower).ok_or_else(|| {
                        format!(
                            "'{word}' is neither an attribute (bold, dim, italic, ul, reverse, \
                             strike) nor a colour (a name such as red or brightred, normal, \
                             0-255, #rrggbb)"
                        )
                    })?;
                    match colors {
                        0 => style.fg = color,
                        1 => style.bg = color,
                        _ => {
                            return Err(format!(
                                "'{word}' is a third colour: the first is the text's, \
                                 the second the background's"
                            ));
                        }
                    }
                    colors += 1;
                }
            }
        }
        Ok(style)
    }

    /// The style with 24-bit colours replaced by the nearest entries of the
    /// 256-colour palette, for terminals that cannot show them.
    pub fn to_256_colors(self) -> Style {
        let fit = |color: Option<Color>| match color {
            Some(Color::Rgb(r, g, b)) => {
                Some(Color::Fixed(ansi_colours::ansi256_from_rgb((r, g, b))))
            }
            other => other,
        };
        Style {
            fg: fit(self.fg),
            bg: fit(self.bg),
            ..self
        }
    }

    pub fn is_plain(&self) -> bool {
        *self == Style::PLAIN
    }

    /// Whether spaces drawn in this style are visible, so trailing ones must
    /// not be trimmed.
    pub fn shows_blanks(&self) -> bool {
        self.bg.is_some() || self.reverse || self.underline || self.strike
    }

    /// The escape sequence that switches from whatever was active to exactly
    /// this style: it starts with a reset, so no attribute leaks through.
    pub fn sgr(&self) -> String {
        let mut s = String::from("\x1b[0");
        for (on, code) in [
            (self.bold, 1),
            (self.dim, 2),
            (self.italic, 3),
            (self.underline, 4),
            (self.reverse, 7),
            (self.strike, 9),
        ] {
            if on {
                let _ = write!(s, ";{code}");
            }
        }
        if let Some(color) = self.fg {
            push_color(&mut s, color, false);
        }
        if let Some(color) = self.bg {
            push_color(&mut s, color, true);
        }
        s.push('m');
        s
    }
}

/// Written back in the words `parse` reads.
impl fmt::Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut words: Vec<String> = [
            (self.bold, "bold"),
            (self.dim, "dim"),
            (self.italic, "italic"),
            (self.underline, "ul"),
            (self.reverse, "reverse"),
            (self.strike, "strike"),
        ]
        .into_iter()
        .filter(|&(on, _)| on)
        .map(|(_, word)| word.to_string())
        .collect();
        if self.fg.is_some() || self.bg.is_some() {
            words.push(self.fg.map_or_else(|| "normal".to_string(), color_name));
        }
        words.extend(self.bg.map(color_name));
        if words.is_empty() {
            words.push("normal".to_string());
        }
        f.write_str(&words.join(" "))
    }
}

/// `None` stands for `normal`, the terminal's own colour.
fn parse_color(word: &str) -> Option<Option<Color>> {
    if word == "normal" {
        return Some(None);
    }
    let (bright, name) = match word.strip_prefix("bright") {
        Some(name) => (8, name),
        None => (0, word),
    };
    if let Some(n) = NAMES.iter().position(|&known| known == name) {
        return Some(Some(Color::Ansi(n as u8 + bright)));
    }
    if let Some(hex) = word.strip_prefix('#') {
        return parse_hex(hex).map(Some);
    }
    let n: u8 = word.parse().ok()?;
    Some(Some(if n < 16 {
        Color::Ansi(n)
    } else {
        Color::Fixed(n)
    }))
}

/// `rrggbb`, or `rgb` with each digit doubled.
fn parse_hex(hex: &str) -> Option<Color> {
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |i: usize, len: usize| u8::from_str_radix(&hex[i * len..(i + 1) * len], 16).ok();
    match hex.len() {
        6 => Some(Color::Rgb(channel(0, 2)?, channel(1, 2)?, channel(2, 2)?)),
        3 => Some(Color::Rgb(
            channel(0, 1)? * 17,
            channel(1, 1)? * 17,
            channel(2, 1)? * 17,
        )),
        _ => None,
    }
}

fn color_name(color: Color) -> String {
    match color {
        Color::Ansi(n) if n < 8 => NAMES[usize::from(n)].to_string(),
        Color::Ansi(n) => format!("bright{}", NAMES[usize::from(n & 7)]),
        Color::Fixed(n) => n.to_string(),
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
    }
}

fn push_color(s: &mut String, color: Color, background: bool) {
    let _ = match color {
        Color::Ansi(n) if n < 8 => {
            write!(s, ";{}", u32::from(n) + if background { 40 } else { 30 })
        }
        Color::Ansi(n) => write!(
            s,
            ";{}",
            u32::from(n & 7) + if background { 100 } else { 90 }
        ),
        Color::Fixed(n) => write!(s, ";{};5;{n}", if background { 48 } else { 38 }),
        Color::Rgb(r, g, b) => write!(s, ";{};2;{r};{g};{b}", if background { 48 } else { 38 }),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sgr_starts_with_a_reset() {
        assert_eq!(Style::PLAIN.sgr(), "\x1b[0m");
        assert_eq!(Style::fg(GREEN).bold().sgr(), "\x1b[0;1;32m");
    }

    #[test]
    fn bright_and_extended_colours() {
        assert_eq!(Style::fg(BRIGHT_BLUE).sgr(), "\x1b[0;94m");
        assert_eq!(Style::PLAIN.on(YELLOW).sgr(), "\x1b[0;43m");
        assert_eq!(Style::fg(Color::Fixed(236)).sgr(), "\x1b[0;38;5;236m");
        assert_eq!(Style::fg(Color::Rgb(1, 2, 3)).sgr(), "\x1b[0;38;2;1;2;3m");
    }

    #[test]
    fn styles_are_read_as_git_writes_them() {
        let parse = |spec| Style::parse(spec).unwrap();
        assert_eq!(
            parse("bold brightmagenta"),
            Style::fg(BRIGHT_MAGENTA).bold()
        );
        assert_eq!(parse("Yellow, UL"), Style::fg(YELLOW).underline());
        assert_eq!(parse("black yellow"), Style::fg(BLACK).on(YELLOW));
        assert_eq!(parse("normal blue"), Style::PLAIN.on(BLUE));
        assert_eq!(parse("dim strike"), Style::PLAIN.dim().strike());
        assert_eq!(parse("italic reverse"), Style::PLAIN.italic().reverse());
        assert_eq!(parse("3 12"), Style::fg(YELLOW).on(BRIGHT_BLUE));
        assert_eq!(parse("208").fg, Some(Color::Fixed(208)));
        assert_eq!(parse("#ff8000").fg, Some(Color::Rgb(255, 128, 0)));
        assert_eq!(parse("#f80").fg, Some(Color::Rgb(255, 136, 0)));
        assert_eq!(parse(""), Style::PLAIN);
        assert_eq!(parse("normal"), Style::PLAIN);
    }

    #[test]
    fn style_words_that_make_no_sense_are_refused() {
        for spec in [
            "blod",
            "brightnormal",
            "256",
            "#ff80",
            "#gg0000",
            "red green blue",
        ] {
            assert!(Style::parse(spec).is_err(), "{spec}");
        }
    }

    #[test]
    fn styles_are_written_back_in_the_same_words() {
        for spec in [
            "bold brightmagenta",
            "black yellow",
            "normal blue",
            "dim strike",
            "ul 208",
            "#ff8000",
            "normal",
        ] {
            assert_eq!(Style::parse(spec).unwrap().to_string(), spec);
        }
    }

    #[test]
    fn rgb_becomes_the_nearest_palette_entry() {
        let style = Style::fg(Color::Rgb(255, 135, 0)).on(Color::Ansi(1)).bold();
        let fitted = style.to_256_colors();
        assert_eq!(fitted.fg, Some(Color::Fixed(208)));
        assert_eq!(fitted.bg, Some(Color::Ansi(1)));
        assert!(fitted.bold);
    }

    #[test]
    fn patch_keeps_attributes_and_overrides_colour() {
        let outer = Style::fg(GREEN).bold();
        let inner = Style::fg(YELLOW).underline();
        let both = outer.patch(inner);
        assert_eq!(both.fg, Some(YELLOW));
        assert!(both.bold && both.underline);
    }
}
