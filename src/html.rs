//! Just enough HTML for what Markdown files embed: tags split from text,
//! attributes read, entities decoded. What each tag does to the output is the
//! renderer's business.

pub enum Token<'a> {
    Text(&'a str),
    Tag(Tag),
    /// A comment, doctype or processing instruction — never shown.
    Hidden,
}

#[derive(Debug, PartialEq)]
pub struct Tag {
    /// Lowercase.
    pub name: String,
    pub closing: bool,
    pub self_closing: bool,
    pub attrs: Vec<(String, String)>,
    pub raw: String,
}

impl Tag {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Splits an HTML fragment into text, tags and hidden parts.
pub fn tokenize(s: &str) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let mut rest = s;
    while !rest.is_empty() {
        let Some(lt) = rest.find('<') else {
            out.push(Token::Text(rest));
            break;
        };
        if lt > 0 {
            out.push(Token::Text(&rest[..lt]));
            rest = &rest[lt..];
        }
        match markup_len(rest) {
            Some(len) => {
                let raw = &rest[..len];
                out.push(match parse_tag(raw) {
                    Some(tag) => Token::Tag(tag),
                    None => Token::Hidden,
                });
                rest = &rest[len..];
            }
            None => {
                out.push(Token::Text("<"));
                rest = &rest[1..];
            }
        }
    }
    out
}

/// Length of the comment, declaration or tag at the start of `s`, if it is
/// one.
fn markup_len(s: &str) -> Option<usize> {
    if s.starts_with("<!--") {
        return Some(s.find("-->").map_or(s.len(), |i| i + 3));
    }
    if s.starts_with("<!") || s.starts_with("<?") {
        return Some(s.find('>').map_or(s.len(), |i| i + 1));
    }
    let after = s[1..].trim_start_matches('/');
    if !after.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }
    let mut quote = None;
    for (i, c) in s.char_indices().skip(1) {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), _) if c == q => quote = None,
            (None, '>') => return Some(i + 1),
            (None, '<') => return None,
            _ => {}
        }
    }
    None
}

/// Reads one tag such as `<a href="x">`, `</b>` or `<br/>`. Comments and
/// declarations give `None`.
pub fn parse_tag(raw: &str) -> Option<Tag> {
    let inner = raw.strip_prefix('<')?.strip_suffix('>')?;
    let (closing, inner) = match inner.strip_prefix('/') {
        Some(rest) => (true, rest),
        None => (false, inner),
    };
    let (self_closing, inner) = match inner.strip_suffix('/') {
        Some(rest) => (true, rest),
        None => (false, inner),
    };
    let name_len = inner
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == ':'))
        .unwrap_or(inner.len());
    if name_len == 0 || !inner.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }
    let name = inner[..name_len].to_ascii_lowercase();
    Some(Tag {
        name,
        closing,
        self_closing,
        attrs: parse_attrs(&inner[name_len..]),
        raw: raw.to_string(),
    })
}

fn parse_attrs(mut s: &str) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    loop {
        s = s.trim_start();
        if s.is_empty() {
            break;
        }
        let name_len = s
            .find(|c: char| c.is_whitespace() || c == '=')
            .unwrap_or(s.len());
        if name_len == 0 {
            // A stray '=': skip it.
            s = &s[1..];
            continue;
        }
        let name = s[..name_len].to_ascii_lowercase();
        s = s[name_len..].trim_start();
        let mut value = String::new();
        if let Some(rest) = s.strip_prefix('=') {
            let rest = rest.trim_start();
            if let Some(q) = rest.chars().next().filter(|c| *c == '"' || *c == '\'') {
                let body = &rest[1..];
                let end = body.find(q).unwrap_or(body.len());
                value = decode_entities(&body[..end]);
                s = body.get(end + 1..).unwrap_or("");
            } else {
                let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
                value = decode_entities(&rest[..end]);
                s = &rest[end..];
            }
        }
        attrs.push((name, value));
    }
    attrs
}

/// Replaces character references (`&amp;`, `&#169;`, `&#xA9;`) with the
/// characters they stand for. Unknown names stay as written.
pub fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let decoded = rest[1..]
            .find(';')
            .filter(|&end| end > 0 && end <= 32)
            .and_then(|end| entity(&rest[1..=end]).map(|c| (c, end + 2)));
        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn entity(name: &str) -> Option<char> {
    if let Some(num) = name.strip_prefix('#') {
        let code = match num.strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
            None => num.parse().ok()?,
        };
        return char::from_u32(code).filter(|c| *c != '\0');
    }
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => '\u{a0}',
        "ensp" => '\u{2002}',
        "emsp" => '\u{2003}',
        "thinsp" => '\u{2009}',
        "zwj" => '\u{200d}',
        "zwnj" => '\u{200c}',
        "copy" => '©',
        "reg" => '®',
        "trade" => '™',
        "hellip" => '…',
        "mdash" => '—',
        "ndash" => '–',
        "laquo" => '«',
        "raquo" => '»',
        "lsquo" => '‘',
        "rsquo" => '’',
        "ldquo" => '“',
        "rdquo" => '”',
        "bull" => '•',
        "middot" => '·',
        "times" => '×',
        "divide" => '÷',
        "plusmn" => '±',
        "deg" => '°',
        "sect" => '§',
        "para" => '¶',
        "larr" => '←',
        "rarr" => '→',
        "uarr" => '↑',
        "darr" => '↓',
        "harr" => '↔',
        "check" => '✓',
        "euro" => '€',
        "pound" => '£',
        "yen" => '¥',
        "cent" => '¢',
        "le" => '≤',
        "ge" => '≥',
        "ne" => '≠',
        "infin" => '∞',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_with_attributes() {
        let tag = parse_tag(r#"<img src="a.png" alt='A &amp; B' width=10>"#).unwrap();
        assert_eq!(tag.name, "img");
        assert_eq!(tag.attr("src"), Some("a.png"));
        assert_eq!(tag.attr("alt"), Some("A & B"));
        assert_eq!(tag.attr("width"), Some("10"));
        let close = parse_tag("</KBD>").unwrap();
        assert!(close.closing && close.name == "kbd");
        assert!(parse_tag("<br/>").unwrap().self_closing);
        assert!(parse_tag("<!-- note -->").is_none());
    }

    #[test]
    fn tokenizer_separates_text_and_markup() {
        let tokens = tokenize("a <b>bold</b> <!-- x --> 1 < 2");
        let kinds: Vec<&str> = tokens
            .iter()
            .map(|t| match t {
                Token::Text(_) => "text",
                Token::Tag(_) => "tag",
                Token::Hidden => "hidden",
            })
            .collect();
        assert_eq!(
            kinds,
            [
                "text", "tag", "text", "tag", "text", "hidden", "text", "text", "text"
            ]
        );
    }

    #[test]
    fn entities() {
        assert_eq!(
            decode_entities("a&amp;b &#169; &#x41; &bogus; & x"),
            "a&b © A &bogus; & x"
        );
    }
}
