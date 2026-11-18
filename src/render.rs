//! Turns a Markdown document into styled lines.

use std::rc::Rc;

use comrak::nodes::{AlertType, ListDelimType, ListType, NodeList, NodeValue, TableAlignment};
use comrak::{Arena, Node, Options, parse_document};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::code::Highlighter;
use crate::html::{self, Tag, Token};
use crate::style::Style;
use crate::table::{self, Align};
use crate::text::{Line, Span, cut, plain_text, prefix_lines, trim_end, wrap};
use crate::theme::{GLYPHS, Glyphs, Theme};

pub struct Settings {
    /// Columns available for the text.
    pub width: usize,
    /// Emit OSC 8 hyperlinks.
    pub hyperlinks: bool,
    /// Draw emphasis with the italic attribute as well as its colour.
    pub italic: bool,
    /// The terminal takes 24-bit colours (otherwise code colours are
    /// mapped to the 256-colour palette).
    pub true_color: bool,
    /// Syntax theme for code blocks.
    pub code_theme: String,
}

/// The Markdown dialect: GitHub's, which is what most files are written for.
fn options() -> Options<'static> {
    let mut options = Options::default();
    let ext = &mut options.extension;
    ext.strikethrough = true;
    ext.table = true;
    ext.autolink = true;
    ext.tasklist = true;
    ext.footnotes = true;
    ext.alerts = true;
    ext.math_dollars = true;
    ext.math_dollars_block = true;
    ext.math_code = true;
    ext.description_lists = true;
    ext.front_matter_delimiter = Some("---".to_string());
    options
}

pub fn render(markdown: &str, settings: &Settings) -> Vec<Line> {
    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &options());
    let mut renderer = Renderer {
        settings,
        theme: Theme::new(settings.italic),
        glyphs: &GLYPHS,
        highlighter: None,
        base: Style::PLAIN,
        list_depth: 0,
        footnotes: 0,
    };
    renderer.blocks(root, settings.width.max(1), false)
}

struct Renderer<'s> {
    settings: &'s Settings,
    theme: Theme,
    glyphs: &'static Glyphs,
    /// Loaded on the first code block that names a language.
    highlighter: Option<Highlighter>,
    /// The style text starts from; block quotes and table headers change it.
    base: Style,
    list_depth: usize,
    /// Footnote definitions drawn so far.
    footnotes: usize,
}

impl Renderer<'_> {
    // ---- blocks -------------------------------------------------------

    /// The child blocks of `parent`, one after another — with a blank line
    /// between them unless `tight` (the items of a tight list).
    fn blocks(&mut self, parent: Node<'_>, width: usize, tight: bool) -> Vec<Line> {
        let mut out = Vec::new();
        for child in parent.children() {
            let first_footnote = self.footnotes == 0
                && matches!(child.data().value, NodeValue::FootnoteDefinition(_));
            let lines = self.block(child, width);
            if lines.is_empty() {
                continue;
            }
            if !out.is_empty() && !tight {
                out.push(Line::new());
            }
            if first_footnote {
                let rule = repeat(
                    self.glyphs.footnote_rule,
                    (width / 3).clamp(10, 30).min(width),
                );
                out.push(Line::plain(rule, self.theme.rule));
            }
            out.extend(lines);
        }
        out
    }

    fn block(&mut self, node: Node<'_>, width: usize) -> Vec<Line> {
        let data = node.data();
        match &data.value {
            NodeValue::Paragraph => self.paragraph(node, width),
            NodeValue::Heading(heading) => self.heading(node, heading.level, width),
            NodeValue::BlockQuote | NodeValue::MultilineBlockQuote(_) => self.quote(node, width),
            NodeValue::Alert(alert) => {
                self.alert(node, alert.alert_type, alert.title.as_deref(), width)
            }
            NodeValue::List(list) => self.list(node, list, width),
            NodeValue::CodeBlock(code) => self.code_block(&code.info, &code.literal, None, width),
            NodeValue::MathBlock(math) => self.code_block("", &math.literal, Some("math"), width),
            NodeValue::HtmlBlock(block) => self.html_block(&block.literal, width),
            NodeValue::ThematicBreak(_) => vec![Line::plain(
                repeat(self.glyphs.rule, width),
                self.theme.rule,
            )],
            NodeValue::FootnoteDefinition(_) => self.footnote(node, width),
            NodeValue::Table(table) => self.table(node, &table.alignments, width),
            NodeValue::DescriptionList => self.description_list(node, width),
            NodeValue::FrontMatter(text) => self.front_matter(text, width),
            _ => self.blocks(node, width, false),
        }
    }

    fn paragraph(&mut self, node: Node<'_>, width: usize) -> Vec<Line> {
        let spans = self.inlines(node, self.base);
        if plain_text(&spans).trim().is_empty() {
            return Vec::new();
        }
        wrap(&spans, width)
    }

    fn heading(&mut self, node: Node<'_>, level: u8, width: usize) -> Vec<Line> {
        let heading = &self.theme.headings[usize::from(level.clamp(1, 6)) - 1];
        let (style, rule) = (heading.text, heading.rule);
        let spans = self.inlines(node, self.base.patch(style));
        let mut lines = wrap(&spans, width);
        if let Some((c, rule_style)) = rule {
            lines.push(Line::plain(repeat(c, width), rule_style));
        }
        lines
    }

    fn quote(&mut self, node: Node<'_>, width: usize) -> Vec<Line> {
        let saved = self.base;
        self.base = self.base.patch(self.theme.quote_text);
        let inner = self.blocks(node, width.saturating_sub(2).max(1), false);
        self.base = saved;
        let bar = Line::plain(format!("{} ", self.glyphs.quote_bar), self.theme.quote_bar);
        prefix_lines(inner, &bar, &bar)
    }

    /// GitHub's `> [!NOTE]` boxes.
    fn alert(
        &mut self,
        node: Node<'_>,
        kind: AlertType,
        title: Option<&str>,
        width: usize,
    ) -> Vec<Line> {
        let (index, name) = match kind {
            AlertType::Note => (0, "Note"),
            AlertType::Tip => (1, "Tip"),
            AlertType::Important => (2, "Important"),
            AlertType::Warning => (3, "Warning"),
            AlertType::Caution => (4, "Caution"),
            _ => (0, "Note"),
        };
        let color = self.theme.alerts[index];
        let title = title
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .unwrap_or(name);
        let mut lines = wrap(
            &[Span::new(title, color.bold())],
            width.saturating_sub(2).max(1),
        );
        lines.extend(self.blocks(node, width.saturating_sub(2).max(1), false));
        let bar = Line::plain(format!("{} ", self.glyphs.alert_bar), color);
        prefix_lines(lines, &bar, &bar)
    }

    fn list(&mut self, node: Node<'_>, list: &NodeList, width: usize) -> Vec<Line> {
        let ordered = list.list_type == ListType::Ordered;
        let delimiter = if list.delimiter == ListDelimType::Paren {
            ")"
        } else {
            "."
        };
        let items: Vec<Node<'_>> = node.children().collect();
        let numbers: Vec<String> = (0..items.len())
            .map(|i| format!("{}{delimiter}", list.start + i))
            .collect();
        let number_width = numbers.iter().map(String::len).max().unwrap_or(0);
        let bullet = self.glyphs.bullets[self.list_depth % self.glyphs.bullets.len()];

        self.list_depth += 1;
        let mut out = Vec::new();
        for (i, item) in items.iter().enumerate() {
            let task = match &item.data().value {
                NodeValue::TaskItem(task) => Some(task.symbol.is_some()),
                _ => None,
            };
            let mut marker = Line::new();
            if ordered {
                marker.push_str(
                    &format!("{:>number_width$} ", numbers[i]),
                    self.theme.list_marker,
                );
            }
            match task {
                Some(done) => {
                    let (glyph, style) = if done {
                        (self.glyphs.task_done, self.theme.task_done)
                    } else {
                        (self.glyphs.task_open, self.theme.task_open)
                    };
                    marker.push_str(glyph, style);
                    marker.push_str(" ", Style::PLAIN);
                }
                None if !ordered => {
                    marker.push_str(bullet, self.theme.list_marker);
                    marker.push_str(" ", Style::PLAIN);
                }
                None => {}
            }
            let indent = marker.width();
            let body = self.blocks(item, width.saturating_sub(indent).max(1), list.tight);
            if i > 0 && !list.tight {
                out.push(Line::new());
            }
            if body.is_empty() {
                out.push(trim_end(marker));
            } else {
                out.extend(prefix_lines(body, &marker, &spaces(indent)));
            }
        }
        self.list_depth -= 1;
        out
    }

    fn footnote(&mut self, node: Node<'_>, width: usize) -> Vec<Line> {
        // Definitions come at the end of the document in the order they are
        // referenced, so counting them gives the numbers the references show.
        self.footnotes += 1;
        let mut label = Line::plain(format!("[{}]", self.footnotes), self.theme.footnote);
        label.push_str(" ", Style::PLAIN);
        let indent = label.width();
        let body = self.blocks(node, width.saturating_sub(indent).max(1), false);
        prefix_lines(body, &label, &spaces(indent))
    }

    fn table(&mut self, node: Node<'_>, alignments: &[TableAlignment], width: usize) -> Vec<Line> {
        let mut rows = Vec::new();
        let mut header_rows = 0;
        for row in node.children() {
            let header = matches!(&row.data().value, NodeValue::TableRow(r) if r.header);
            if header {
                header_rows += 1;
            }
            let base = if header {
                self.base.patch(self.theme.table_header)
            } else {
                self.base
            };
            let cells: Vec<Vec<Span>> = row
                .children()
                .map(|cell| self.inlines(cell, base))
                .collect();
            rows.push(cells);
        }
        let aligns: Vec<Align> = alignments
            .iter()
            .map(|a| match a {
                TableAlignment::Center => Align::Center,
                TableAlignment::Right => Align::Right,
                _ => Align::Left,
            })
            .collect();
        table::render(
            &rows,
            header_rows,
            &aligns,
            width,
            self.theme.table_border,
            &self.glyphs.table,
        )
    }

    fn description_list(&mut self, node: Node<'_>, width: usize) -> Vec<Line> {
        let mut out = Vec::new();
        for item in node.children() {
            if !out.is_empty() {
                out.push(Line::new());
            }
            for part in item.children() {
                if matches!(part.data().value, NodeValue::DescriptionTerm) {
                    let saved = self.base;
                    self.base = self.base.patch(self.theme.term);
                    out.extend(self.blocks(part, width, true));
                    self.base = saved;
                } else {
                    let body = self.blocks(part, width.saturating_sub(4).max(1), false);
                    out.extend(prefix_lines(body, &spaces(4), &spaces(4)));
                }
            }
        }
        out
    }

    fn front_matter(&mut self, raw: &str, width: usize) -> Vec<Line> {
        let lines: Vec<&str> = raw.trim_end().lines().collect();
        let inner = if lines.len() > 2 {
            lines[1..lines.len() - 1].join("\n")
        } else {
            String::new()
        };
        self.code_block("yaml", &inner, Some("front matter"), width)
    }

    /// A fenced or indented code block, framed, with the language named in
    /// the top line and the code highlighted when the language is known.
    fn code_block(
        &mut self,
        info: &str,
        literal: &str,
        label: Option<&str>,
        width: usize,
    ) -> Vec<Line> {
        let language = info
            .trim()
            .trim_start_matches('{')
            .trim_start_matches('.')
            .split(|c: char| c.is_whitespace() || c == ',' || c == '{' || c == '}')
            .next()
            .unwrap_or("")
            .to_string();
        let code = expand_tabs(literal.trim_end_matches('\n'), 4);

        let mut lines: Vec<Vec<Span>> = Vec::new();
        if !language.is_empty() && !code.is_empty() {
            let highlighter = self.highlighter();
            if let Some(syntax) = highlighter.find(&language) {
                lines = highlighter.highlight(&code, syntax);
                for spans in &mut lines {
                    for span in spans.iter_mut() {
                        span.text = clean(&span.text);
                    }
                }
            }
        }
        if lines.is_empty() && !code.is_empty() {
            let style = if label == Some("math") {
                self.theme.math
            } else {
                self.theme.code_text
            };
            lines = code
                .split('\n')
                .map(|line| vec![Span::new(clean(line), style)])
                .collect();
        }

        let frame = self.theme.code_frame;
        let chars = &self.glyphs.code;
        let mut top = Line::plain(format!("{}{}", chars.top, chars.line), frame);
        let name = label
            .map(str::to_string)
            .or_else(|| Some(language.clone()).filter(|l| !l.is_empty()));
        if let Some(name) = name {
            top.push_str(" ", frame);
            top.push_str(&name, self.theme.code_label);
            top.push_str(" ", frame);
        }
        let fill = width.saturating_sub(top.width());
        top.push_str(&repeat(chars.line, fill), frame);

        let mut out = vec![top];
        let side = Line::plain(format!("{} ", chars.side), frame);
        let inner = width.saturating_sub(2).max(1);
        for spans in lines {
            for (piece, _) in cut(&spans, inner) {
                out.push(piece.after(&side));
            }
        }
        out.push(Line::plain(
            format!(
                "{}{}",
                chars.bottom,
                repeat(chars.line, width.saturating_sub(1))
            ),
            frame,
        ));
        out
    }

    fn highlighter(&mut self) -> &Highlighter {
        let settings = self.settings;
        self.highlighter.get_or_insert_with(|| {
            Highlighter::new(&settings.code_theme, settings.true_color, settings.italic)
        })
    }

    /// An HTML block: tags become formatting and line breaks, comments
    /// vanish, the text is shown.
    fn html_block(&mut self, literal: &str, width: usize) -> Vec<Line> {
        let mut spans = Vec::new();
        let mut state = HtmlState {
            block: true,
            ..HtmlState::default()
        };
        let mut skipping: Option<String> = None;
        for token in html::tokenize(literal) {
            match token {
                Token::Hidden => {}
                Token::Text(_) if skipping.is_some() => {}
                Token::Text(text) => {
                    let text = collapse_whitespace(&html::decode_entities(text));
                    let span = Span::linked(text, state.style(self.base), state.link());
                    if span.text == " " {
                        // Just the gap between two tags: it can part two words,
                        // never start a line.
                        if state.written && state.pending_breaks == 0 {
                            spans.push(span);
                        }
                    } else {
                        state.emit(span, &mut spans);
                    }
                }
                Token::Tag(tag) => {
                    if let Some(name) = &skipping {
                        if tag.closing && &tag.name == name {
                            skipping = None;
                        }
                        continue;
                    }
                    if matches!(tag.name.as_str(), "script" | "style" | "template" | "head")
                        && !tag.closing
                        && !tag.self_closing
                    {
                        skipping = Some(tag.name.clone());
                        continue;
                    }
                    self.html_tag(&tag, self.base, None, &mut state, &mut spans);
                }
            }
        }
        if !state.written {
            return Vec::new();
        }
        wrap(&spans, width)
    }

    // ---- inlines ------------------------------------------------------

    fn inlines(&mut self, parent: Node<'_>, style: Style) -> Vec<Span> {
        let mut out = Vec::new();
        self.inline_children(parent, style, None, false, &mut out);
        out
    }

    fn inline_children(
        &mut self,
        parent: Node<'_>,
        style: Style,
        link: Option<&Rc<str>>,
        in_link: bool,
        out: &mut Vec<Span>,
    ) {
        // Inline HTML arrives one tag at a time, as siblings of the text it
        // wraps, so the open tags are tracked across the siblings.
        let mut html = HtmlState::default();
        for child in parent.children() {
            if let NodeValue::HtmlInline(raw) = &child.data().value {
                for token in html::tokenize(raw) {
                    match token {
                        Token::Tag(tag) => self.html_tag(&tag, style, link, &mut html, out),
                        Token::Text(text) => {
                            out.push(Span::linked(text, html.style(style), link.cloned()))
                        }
                        Token::Hidden => {}
                    }
                }
                continue;
            }
            let child_style = html.style(style);
            let child_link = html.link().or_else(|| link.cloned());
            self.inline(
                child,
                child_style,
                child_link.as_ref(),
                in_link || html.in_link(),
                out,
            );
        }
    }

    fn inline(
        &mut self,
        node: Node<'_>,
        style: Style,
        link: Option<&Rc<str>>,
        in_link: bool,
        out: &mut Vec<Span>,
    ) {
        let data = node.data();
        let mut push =
            |text: String, style: Style| out.push(Span::linked(text, style, link.cloned()));
        match &data.value {
            NodeValue::Text(text) => push(clean(text), style),
            NodeValue::SoftBreak => push(" ".to_string(), style),
            NodeValue::LineBreak => out.push(Span::new("\n", Style::PLAIN)),
            NodeValue::Code(code) => push(clean(&code.literal), style.patch(self.theme.code)),
            NodeValue::Math(math) => push(clean(&math.literal), style.patch(self.theme.math)),
            NodeValue::FootnoteReference(note) => {
                push(format!("[{}]", note.ix), style.patch(self.theme.footnote));
            }
            NodeValue::Emph(_) => {
                self.inline_children(node, style.patch(self.theme.emph), link, in_link, out);
            }
            NodeValue::Strong(_) => {
                self.inline_children(node, style.patch(self.theme.strong), link, in_link, out);
            }
            NodeValue::Strikethrough(_) => {
                self.inline_children(node, style.patch(self.theme.strike), link, in_link, out);
            }
            NodeValue::Highlight => {
                self.inline_children(node, style.patch(self.theme.mark), link, in_link, out);
            }
            NodeValue::Underline | NodeValue::Insert => {
                self.inline_children(node, style.underline(), link, in_link, out);
            }
            NodeValue::Superscript => {
                push("^".to_string(), style);
                self.inline_children(node, style, link, in_link, out);
            }
            NodeValue::Subscript => {
                push("_".to_string(), style);
                self.inline_children(node, style, link, in_link, out);
            }
            NodeValue::Link(target) => self.link(node, &target.url, style, out),
            NodeValue::Image(target) => self.image(node, &target.url, style, link, in_link, out),
            _ => self.inline_children(node, style, link, in_link, out),
        }
    }

    fn link(&mut self, node: Node<'_>, url: &str, style: Style, out: &mut Vec<Span>) {
        let target = self.hyperlink(url);
        let start = out.len();
        self.inline_children(
            node,
            style.patch(self.theme.link),
            target.as_ref(),
            true,
            out,
        );
        let text = plain_text(&out[start..]);
        if text.trim().is_empty() {
            if !url.is_empty() {
                out.push(Span::linked(url, style.patch(self.theme.link), target));
            }
            return;
        }
        self.push_url(url, &text, style, target, out);
    }

    /// The destination after a link's text, unless the text already shows
    /// it or it only points inside the document.
    fn push_url(
        &self,
        url: &str,
        text: &str,
        style: Style,
        target: Option<Rc<str>>,
        out: &mut Vec<Span>,
    ) {
        if url.is_empty() || url.starts_with('#') || same_destination(text, url) {
            return;
        }
        out.push(Span::new(" ", style));
        out.push(Span::linked(
            format!("({})", clean(url)),
            style.patch(self.theme.url),
            target,
        ));
    }

    fn image(
        &mut self,
        node: Node<'_>,
        url: &str,
        style: Style,
        link: Option<&Rc<str>>,
        in_link: bool,
        out: &mut Vec<Span>,
    ) {
        let mut alt = Vec::new();
        self.inline_children(node, Style::PLAIN, None, true, &mut alt);
        let alt = plain_text(&alt);
        let target = link.cloned().or_else(|| self.hyperlink(url));
        self.push_image(alt.trim(), url, style, target, in_link, out);
    }

    fn push_image(
        &self,
        alt: &str,
        url: &str,
        style: Style,
        target: Option<Rc<str>>,
        in_link: bool,
        out: &mut Vec<Span>,
    ) {
        let label = if alt.is_empty() {
            "[image]".to_string()
        } else {
            format!("[image: {alt}]")
        };
        out.push(Span::linked(
            label,
            style.patch(self.theme.image),
            target.clone(),
        ));
        // Inside a link (a badge, say) the link's own address is what
        // matters; the picture's is noise.
        if !in_link && !url.is_empty() {
            out.push(Span::new(" ", style));
            out.push(Span::linked(
                format!("({})", clean(url)),
                style.patch(self.theme.url),
                target,
            ));
        }
    }

    /// The OSC 8 target for `url`, when hyperlinks are on and the address is
    /// one a terminal can open.
    fn hyperlink(&self, url: &str) -> Option<Rc<str>> {
        if !self.settings.hyperlinks || !has_scheme(url) {
            return None;
        }
        let url: String = url.chars().filter(|c| !c.is_control()).collect();
        Some(Rc::from(url))
    }

    /// One HTML tag, inside a paragraph or in an HTML block (`state.block`).
    fn html_tag(
        &mut self,
        tag: &Tag,
        base: Style,
        outer_link: Option<&Rc<str>>,
        state: &mut HtmlState,
        out: &mut Vec<Span>,
    ) {
        let name = tag.name.as_str();
        let link = |state: &HtmlState| state.link().or_else(|| outer_link.cloned());

        if tag.closing {
            if name == "kbd" {
                state.emit(Span::linked(" ", state.style(base), link(state)), out);
            }
            if let Some(open) = state.close(name) {
                if let Some(href) = open.href {
                    let text = plain_text(&out[open.start..]);
                    self.push_url(&href, &text, state.style(base), open.link, out);
                }
            } else if !is_known_tag(name) {
                let raw = Span::linked(
                    clean(&tag.raw),
                    state.style(base).patch(self.theme.html),
                    link(state),
                );
                state.emit(raw, out);
                return;
            }
            if matches!(name, "td" | "th") {
                state.emit(Span::new("  ", Style::PLAIN), out);
            }
            state.want_breaks(block_breaks(name));
            return;
        }

        state.want_breaks(block_breaks(name));
        let opened = |style: Style| Open {
            name: name.to_string(),
            style,
            link: None,
            href: None,
            start: 0,
        };
        match name {
            "br" if state.block => state.pending_breaks = (state.pending_breaks + 1).min(2),
            "br" => out.push(Span::new("\n", Style::PLAIN)),
            "img" => {
                let alt = tag
                    .attr("alt")
                    .or(tag.attr("title"))
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let src = tag.attr("src").unwrap_or("").to_string();
                let target = link(state).or_else(|| self.hyperlink(&src));
                let in_link = state.in_link() || outer_link.is_some();
                state.start_text(out);
                self.push_image(&alt, &src, state.style(base), target, in_link, out);
            }
            "a" => {
                let href = tag.attr("href").map(str::to_string);
                let target = href.as_deref().and_then(|h| self.hyperlink(h));
                let style = if href.is_some() {
                    self.theme.link
                } else {
                    Style::PLAIN
                };
                state.open(Open {
                    name: "a".to_string(),
                    style,
                    link: target,
                    href,
                    start: out.len(),
                });
            }
            "b" | "strong" => state.open(opened(self.theme.strong)),
            "i" | "em" | "cite" | "dfn" | "var" => state.open(opened(self.theme.emph)),
            "u" | "ins" => state.open(opened(Style::PLAIN.underline())),
            "s" | "del" | "strike" => state.open(opened(self.theme.strike)),
            "code" | "tt" | "samp" => state.open(opened(self.theme.code)),
            "mark" => state.open(opened(self.theme.mark)),
            "kbd" => {
                state.open(opened(self.theme.kbd));
                state.emit(Span::linked(" ", state.style(base), link(state)), out);
            }
            "sup" | "sub" => {
                let mark = if name == "sup" { "^" } else { "_" };
                state.emit(Span::linked(mark, state.style(base), link(state)), out);
                state.open(opened(Style::PLAIN));
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = usize::from(name.as_bytes()[1] - b'0');
                state.open(opened(self.theme.headings[level - 1].text));
            }
            "summary" => {
                state.emit(Span::linked("▸ ", state.style(base), link(state)), out);
                state.open(opened(self.theme.strong));
            }
            "th" => state.open(opened(self.theme.table_header)),
            "li" => {
                let bullet = format!("{} ", self.glyphs.bullets[0]);
                state.emit(Span::new(bullet, self.theme.list_marker), out);
            }
            _ if is_known_tag(name) => {}
            _ => {
                let raw = Span::linked(
                    clean(&tag.raw),
                    state.style(base).patch(self.theme.html),
                    link(state),
                );
                state.emit(raw, out);
            }
        }
        if tag.self_closing && name != "br" && name != "img" {
            state.close(name);
        }
    }
}

/// Where an HTML fragment is at: the tags open at this point of the text
/// and, in an HTML block, the line breaks owed to the next text.
#[derive(Default)]
struct HtmlState {
    open: Vec<Open>,
    /// An HTML block, where block tags break lines; inside a paragraph they
    /// do not.
    block: bool,
    /// 1 for a new line, 2 for a blank line, as the tags passed since the
    /// last text asked for.
    pending_breaks: usize,
    /// Whether any text has been written yet; breaks before it are dropped.
    written: bool,
}

struct Open {
    name: String,
    style: Style,
    link: Option<Rc<str>>,
    /// For `<a href>`: the destination, shown after the text.
    href: Option<String>,
    /// Where the element's text starts in the output.
    start: usize,
}

impl HtmlState {
    fn style(&self, base: Style) -> Style {
        self.open
            .iter()
            .fold(base, |style, open| style.patch(open.style))
    }

    fn link(&self) -> Option<Rc<str>> {
        self.open.iter().rev().find_map(|open| open.link.clone())
    }

    fn in_link(&self) -> bool {
        self.open.iter().any(|open| open.href.is_some())
    }

    fn open(&mut self, open: Open) {
        self.open.push(open);
    }

    fn want_breaks(&mut self, breaks: usize) {
        if self.block {
            self.pending_breaks = self.pending_breaks.max(breaks);
        }
    }

    /// Writes the line breaks owed, ahead of text about to be written.
    fn start_text(&mut self, out: &mut Vec<Span>) {
        if self.pending_breaks > 0 && self.written {
            out.push(Span::new("\n".repeat(self.pending_breaks), Style::PLAIN));
        }
        self.pending_breaks = 0;
        self.written = true;
    }

    fn emit(&mut self, span: Span, out: &mut Vec<Span>) {
        if !span.text.is_empty() {
            self.start_text(out);
            out.push(span);
        }
    }

    /// Closes the innermost open element called `name`, even if elements
    /// opened after it are still open.
    fn close(&mut self, name: &str) -> Option<Open> {
        let i = self.open.iter().rposition(|open| open.name == name)?;
        Some(self.open.remove(i))
    }
}

/// The line breaks a tag puts before and after itself in an HTML block:
/// 1 for a new line, 2 for a blank line, 0 for a tag inside the text.
fn block_breaks(name: &str) -> usize {
    match name {
        "li" | "tr" | "dt" | "dd" | "summary" | "caption" | "figcaption" | "legend" => 1,
        "p" | "div" | "center" | "section" | "article" | "aside" | "header" | "footer" | "nav"
        | "main" | "figure" | "blockquote" | "pre" | "ul" | "ol" | "dl" | "table" | "thead"
        | "tbody" | "tfoot" | "details" | "hr" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
        | "address" | "form" | "fieldset" | "picture" | "video" | "audio" | "iframe" | "html"
        | "body" => 2,
        _ => 0,
    }
}

/// Tags understood well enough to drop when they have nothing to show.
fn is_known_tag(name: &str) -> bool {
    block_breaks(name) > 0
        || matches!(
            name,
            "a" | "abbr"
                | "acronym"
                | "b"
                | "bdi"
                | "bdo"
                | "big"
                | "br"
                | "cite"
                | "code"
                | "col"
                | "colgroup"
                | "data"
                | "del"
                | "dfn"
                | "em"
                | "font"
                | "i"
                | "img"
                | "input"
                | "ins"
                | "kbd"
                | "label"
                | "mark"
                | "meta"
                | "link"
                | "q"
                | "s"
                | "samp"
                | "small"
                | "source"
                | "span"
                | "strike"
                | "strong"
                | "sub"
                | "sup"
                | "svg"
                | "path"
                | "g"
                | "time"
                | "track"
                | "tt"
                | "u"
                | "var"
                | "wbr"
                | "th"
                | "td"
                | "button"
                | "title"
                | "head"
                | "script"
                | "style"
                | "template"
                | "noscript"
        )
}

/// Whether the link text already says where the link goes.
fn same_destination(text: &str, url: &str) -> bool {
    let strip = |s: &str| {
        let s = s.trim();
        let s = s.strip_prefix("mailto:").unwrap_or(s);
        let s = s
            .strip_prefix("https://")
            .or_else(|| s.strip_prefix("http://"))
            .unwrap_or(s);
        s.trim_end_matches('/').to_string()
    };
    strip(text) == strip(url)
}

fn has_scheme(url: &str) -> bool {
    let Some(colon) = url.find(':') else {
        return false;
    };
    let scheme = &url[..colon];
    scheme.len() > 1
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// Text made safe to print: tabs become spaces, other control characters
/// are shown in caret notation (`^[`) instead of reaching the terminal.
fn clean(text: &str) -> String {
    if !text.chars().any(|c| c.is_control()) {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + 8);
    for c in text.chars() {
        match c {
            '\t' => out.push(' '),
            '\n' => out.push('\n'),
            c if (c as u32) < 0x20 => {
                out.push('^');
                out.push(char::from(b'@' + c as u8));
            }
            '\u{7f}' => out.push_str("^?"),
            c if c.is_control() => out.push('\u{fffd}'),
            c => out.push(c),
        }
    }
    out
}

fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = false;
    for c in text.chars() {
        if matches!(c, ' ' | '\t' | '\n' | '\r') {
            space = true;
        } else {
            if space {
                out.push(' ');
                space = false;
            }
            out.push(c);
        }
    }
    if space {
        out.push(' ');
    }
    out
}

/// Replaces tabs by spaces up to the next multiple of `tab` columns.
fn expand_tabs(text: &str, tab: usize) -> String {
    if !text.contains('\t') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + 16);
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let mut column = 0;
        for grapheme in line.graphemes(true) {
            if grapheme == "\t" {
                let n = tab - column % tab;
                out.push_str(&" ".repeat(n));
                column += n;
            } else {
                out.push_str(grapheme);
                column += grapheme.width();
            }
        }
    }
    out
}

fn repeat(c: char, n: usize) -> String {
    std::iter::repeat_n(c, n).collect()
}

fn spaces(n: usize) -> Line {
    Line::plain(" ".repeat(n), Style::PLAIN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_text_that_is_its_own_address() {
        assert!(same_destination("example.com", "https://example.com/"));
        assert!(same_destination("me@example.com", "mailto:me@example.com"));
        assert!(!same_destination("docs", "https://example.com/docs"));
    }

    #[test]
    fn schemes() {
        assert!(has_scheme("https://x"));
        assert!(has_scheme("mailto:a@b"));
        assert!(!has_scheme("docs/a.md"));
        assert!(!has_scheme("C:/x"));
        assert!(!has_scheme("#top"));
    }

    #[test]
    fn control_characters_are_made_visible() {
        assert_eq!(clean("a\x1b[31mb\tc"), "a^[[31mb c");
        assert_eq!(clean("plain"), "plain");
    }

    #[test]
    fn tabs_expand_to_stops() {
        assert_eq!(expand_tabs("a\tb\n\tc", 4), "a   b\n    c");
    }
}
