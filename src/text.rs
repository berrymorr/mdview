//! Styled text: spans, lines, and breaking inline text into lines.

use std::rc::Rc;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::style::Style;

/// A piece of text in one style.
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub text: String,
    pub style: Style,
    /// Target of the OSC 8 hyperlink this text belongs to, if any.
    pub link: Option<Rc<str>>,
}

impl Span {
    pub fn new(text: impl Into<String>, style: Style) -> Span {
        Span {
            text: text.into(),
            style,
            link: None,
        }
    }

    pub fn linked(text: impl Into<String>, style: Style, link: Option<Rc<str>>) -> Span {
        Span {
            text: text.into(),
            style,
            link,
        }
    }

    pub fn width(&self) -> usize {
        self.text.width()
    }
}

/// One output line.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Line {
    pub spans: Vec<Span>,
}

impl Line {
    pub fn new() -> Line {
        Line::default()
    }

    pub fn plain(text: impl Into<String>, style: Style) -> Line {
        let mut line = Line::new();
        line.push(Span::new(text, style));
        line
    }

    pub fn width(&self) -> usize {
        self.spans.iter().map(Span::width).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.spans.iter().all(|s| s.text.is_empty())
    }

    /// Appends a span, merging it into the last one when nothing but the
    /// text differs.
    pub fn push(&mut self, span: Span) {
        if span.text.is_empty() {
            return;
        }
        if let Some(last) = self.spans.last_mut()
            && last.style == span.style
            && last.link == span.link
        {
            last.text.push_str(&span.text);
            return;
        }
        self.spans.push(span);
    }

    pub fn push_str(&mut self, text: &str, style: Style) {
        self.push(Span::new(text, style));
    }

    pub fn append(&mut self, other: Line) {
        for span in other.spans {
            self.push(span);
        }
    }

    /// Pads the line with spaces up to `width` columns.
    pub fn pad_to(&mut self, width: usize, style: Style) {
        let have = self.width();
        if have < width {
            self.push(Span::new(" ".repeat(width - have), style));
        }
    }

    /// `prefix` followed by this line.
    pub fn after(self, prefix: &Line) -> Line {
        let mut line = prefix.clone();
        line.append(self);
        line
    }
}

/// Puts `first` in front of the first line and `rest` in front of every
/// other one — the shape of a list item or a footnote. Empty lines get the
/// prefix with its trailing blanks trimmed.
pub fn prefix_lines(lines: Vec<Line>, first: &Line, rest: &Line) -> Vec<Line> {
    let rest_blank = trim_end(rest.clone());
    lines
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            let prefix = if i == 0 { first } else { rest };
            if line.is_empty() {
                if i == 0 {
                    trim_end(first.clone())
                } else {
                    rest_blank.clone()
                }
            } else {
                line.after(prefix)
            }
        })
        .collect()
}

/// The line without trailing spaces that would not show anyway.
pub fn trim_end(mut line: Line) -> Line {
    while let Some(last) = line.spans.last_mut() {
        if last.style.shows_blanks() {
            break;
        }
        let kept = last.text.trim_end_matches(' ').len();
        last.text.truncate(kept);
        if !last.text.is_empty() {
            break;
        }
        line.spans.pop();
    }
    line
}

enum Token {
    Word(Vec<Span>, usize),
    Space(Span),
}

/// Breaks inline text into lines at most `width` columns wide. Spaces are
/// the break opportunities; a word wider than the whole line is cut where it
/// meets the edge. A `\n` in the text is a hard line break.
pub fn wrap(spans: &[Span], width: usize) -> Vec<Line> {
    let width = width.max(1);
    let mut lines = Vec::new();
    for hard_line in split_hard_breaks(spans) {
        wrap_hard_line(&hard_line, width, &mut lines);
    }
    lines
}

fn split_hard_breaks(spans: &[Span]) -> Vec<Vec<Span>> {
    let mut out = vec![Vec::new()];
    for span in spans {
        let mut parts = span.text.split('\n');
        if let Some(first) = parts.next()
            && !first.is_empty()
        {
            out.last_mut()
                .unwrap()
                .push(Span::linked(first, span.style, span.link.clone()));
        }
        for part in parts {
            out.push(Vec::new());
            if !part.is_empty() {
                out.last_mut()
                    .unwrap()
                    .push(Span::linked(part, span.style, span.link.clone()));
            }
        }
    }
    out
}

fn tokens(spans: &[Span]) -> Vec<Token> {
    let mut out = Vec::new();
    let mut word: Vec<Span> = Vec::new();
    let mut word_width = 0;
    for span in spans {
        let mut rest = span.text.as_str();
        while !rest.is_empty() {
            let is_space = rest.starts_with(' ');
            let end = rest
                .find(|c: char| (c == ' ') != is_space)
                .unwrap_or(rest.len());
            let (chunk, tail) = rest.split_at(end);
            if is_space {
                if !word.is_empty() {
                    out.push(Token::Word(std::mem::take(&mut word), word_width));
                    word_width = 0;
                }
                out.push(Token::Space(Span::linked(
                    chunk,
                    span.style,
                    span.link.clone(),
                )));
            } else {
                // A hyphen between two letters is a place to break too, as
                // in a browser: the next part starts a word of its own, with
                // no space before it.
                let mut part_start = 0;
                for (pos, _) in hyphen_breaks(chunk) {
                    let part = &chunk[part_start..pos];
                    word_width += part.width();
                    word.push(Span::linked(part, span.style, span.link.clone()));
                    out.push(Token::Word(std::mem::take(&mut word), word_width));
                    word_width = 0;
                    part_start = pos;
                }
                let part = &chunk[part_start..];
                word_width += part.width();
                word.push(Span::linked(part, span.style, span.link.clone()));
            }
            rest = tail;
        }
    }
    if !word.is_empty() {
        out.push(Token::Word(word, word_width));
    }
    out
}

/// Byte offsets just after each hyphen that has a letter or digit on both
/// sides.
fn hyphen_breaks(word: &str) -> impl Iterator<Item = (usize, char)> + '_ {
    let chars: Vec<(usize, char)> = word.char_indices().collect();
    (1..chars.len().saturating_sub(1)).filter_map(move |i| {
        let (pos, c) = chars[i];
        (c == '-' && chars[i - 1].1.is_alphanumeric() && chars[i + 1].1.is_alphanumeric())
            .then_some((pos + 1, c))
    })
}

fn wrap_hard_line(spans: &[Span], width: usize, out: &mut Vec<Line>) {
    let start = out.len();
    let mut line = Line::new();
    let mut line_width = 0;
    // Spaces after the last word: they are kept only if another word follows
    // on the same line.
    let mut pending: Vec<Span> = Vec::new();
    let mut pending_width = 0;

    for token in tokens(spans) {
        match token {
            Token::Space(space) => {
                if line_width > 0 {
                    pending_width += space.width();
                    pending.push(space);
                }
            }
            Token::Word(parts, word_width) => {
                if line_width > 0 && line_width + pending_width + word_width <= width {
                    for span in pending.drain(..).chain(parts) {
                        line.push(span);
                    }
                    line_width += pending_width + word_width;
                    pending_width = 0;
                    continue;
                }
                if line_width > 0 {
                    out.push(std::mem::take(&mut line));
                }
                pending.clear();
                pending_width = 0;
                if word_width <= width {
                    for span in parts {
                        line.push(span);
                    }
                    line_width = word_width;
                } else {
                    let mut pieces = cut(&parts, width);
                    let (last, last_width) = pieces.pop().unwrap_or_default();
                    out.extend(pieces.into_iter().map(|(piece, _)| piece));
                    line = last;
                    line_width = last_width;
                }
            }
        }
    }
    if line_width > 0 || out.len() == start {
        out.push(line);
    }
}

/// Cuts text into lines of at most `width` columns, between grapheme
/// clusters, ignoring word boundaries. Returns each line with its width.
pub fn cut(spans: &[Span], width: usize) -> Vec<(Line, usize)> {
    let width = width.max(1);
    let mut out = Vec::new();
    let mut line = Line::new();
    let mut line_width = 0;
    for span in spans {
        let mut piece = String::new();
        for grapheme in span.text.graphemes(true) {
            let w = grapheme.width();
            if line_width + w > width && line_width > 0 {
                line.push(Span::linked(
                    std::mem::take(&mut piece),
                    span.style,
                    span.link.clone(),
                ));
                out.push((std::mem::take(&mut line), line_width));
                line_width = 0;
            }
            piece.push_str(grapheme);
            line_width += w;
        }
        line.push(Span::linked(piece, span.style, span.link.clone()));
    }
    if line_width > 0 || out.is_empty() {
        out.push((line, line_width));
    }
    out
}

/// The plain text of a run of spans.
pub fn plain_text(spans: &[Span]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

/// Width of the text laid out without wrapping: its widest hard line.
pub fn natural_width(spans: &[Span]) -> usize {
    split_hard_breaks(spans)
        .iter()
        .map(|line| line.iter().map(Span::width).sum())
        .max()
        .unwrap_or(0)
}

/// Writes lines as terminal output: with SGR escapes and OSC 8 hyperlinks
/// when `color` is on, as bare text otherwise. Each line ends with every
/// attribute switched off, so a pager can show any line on its own.
pub fn write_lines(lines: Vec<Line>, color: bool, out: &mut String) {
    for line in lines {
        let line = trim_end(line);
        if color {
            write_styled(&line, out);
        } else {
            for span in &line.spans {
                out.push_str(&span.text);
            }
        }
        out.push('\n');
    }
}

fn write_styled(line: &Line, out: &mut String) {
    const LINK_END: &str = "\x1b]8;;\x1b\\";
    let mut style = Style::PLAIN;
    let mut link: Option<&Rc<str>> = None;
    for span in &line.spans {
        if span.link.as_ref() != link {
            if link.is_some() {
                out.push_str(LINK_END);
            }
            if let Some(url) = &span.link {
                out.push_str("\x1b]8;;");
                out.push_str(url);
                out.push_str("\x1b\\");
            }
            link = span.link.as_ref();
        }
        if span.style != style {
            out.push_str(&span.style.sgr());
            style = span.style;
        }
        out.push_str(&span.text);
    }
    if link.is_some() {
        out.push_str(LINK_END);
    }
    if !style.is_plain() {
        out.push_str("\x1b[0m");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(lines: &[Line]) -> Vec<String> {
        lines.iter().map(|l| plain_text(&l.spans)).collect()
    }

    fn plain(s: &str) -> Vec<Span> {
        vec![Span::new(s, Style::PLAIN)]
    }

    #[test]
    fn wraps_at_spaces() {
        let lines = wrap(&plain("the quick brown fox jumps"), 10);
        assert_eq!(texts(&lines), ["the quick", "brown fox", "jumps"]);
    }

    #[test]
    fn keeps_styles_across_a_word() {
        let spans = vec![
            Span::new("ab", Style::PLAIN.bold()),
            Span::new("cd ef", Style::PLAIN),
        ];
        let lines = wrap(&spans, 4);
        assert_eq!(texts(&lines), ["abcd", "ef"]);
        assert_eq!(lines[0].spans.len(), 2);
    }

    #[test]
    fn cuts_words_wider_than_the_line() {
        let lines = wrap(&plain("a https://example.com/long b"), 8);
        assert_eq!(
            texts(&lines),
            ["a", "https://", "example.", "com/long", "b"]
        );
    }

    #[test]
    fn breaks_after_hyphens_between_letters() {
        let lines = wrap(&plain("a well-known self-contained fact"), 12);
        assert_eq!(
            texts(&lines),
            ["a well-known", "self-", "contained", "fact"]
        );
        let lines = wrap(&plain("--no-paging"), 6);
        assert_eq!(texts(&lines), ["--no-", "paging"]);
    }

    #[test]
    fn hard_breaks_and_blank_lines() {
        let lines = wrap(&plain("one\n\ntwo"), 10);
        assert_eq!(texts(&lines), ["one", "", "two"]);
    }

    #[test]
    fn wide_characters_count_double() {
        let lines = wrap(&plain("日本語のテキスト"), 6);
        assert_eq!(texts(&lines), ["日本語", "のテキ", "スト"]);
    }

    #[test]
    fn no_trailing_or_leading_spaces() {
        let lines = wrap(&plain("aaa   bbb"), 4);
        assert_eq!(texts(&lines), ["aaa", "bbb"]);
    }

    #[test]
    fn prefixes_trim_on_blank_lines() {
        let lines = vec![Line::plain("x", Style::PLAIN), Line::new()];
        let first = Line::plain("> ", Style::PLAIN);
        let out = prefix_lines(lines, &first, &first);
        assert_eq!(texts(&out), ["> x", ">"]);
    }

    #[test]
    fn styled_output_resets_at_line_end() {
        let mut line = Line::plain("a", Style::PLAIN.bold());
        line.push(Span::linked("b", Style::PLAIN, Some(Rc::from("https://x"))));
        let mut out = String::new();
        write_lines(vec![line.clone()], true, &mut out);
        assert_eq!(
            out,
            "\x1b[0;1ma\x1b]8;;https://x\x1b\\\x1b[0mb\x1b]8;;\x1b\\\n"
        );
        let mut bare = String::new();
        write_lines(vec![line], false, &mut bare);
        assert_eq!(bare, "ab\n");
    }
}
