//! Text attributes and colours, and the SGR escape sequences they become.

use std::fmt::Write as _;

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
    fn patch_keeps_attributes_and_overrides_colour() {
        let outer = Style::fg(GREEN).bold();
        let inner = Style::fg(YELLOW).underline();
        let both = outer.patch(inner);
        assert_eq!(both.fg, Some(YELLOW));
        assert!(both.bold && both.underline);
    }
}
