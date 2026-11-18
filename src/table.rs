//! GFM tables: column widths that fit the screen, wrapped cells, borders.

use crate::style::Style;
use crate::text::{Line, Span, natural_width, wrap};
use crate::theme::BoxChars;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// Narrowest a column is squeezed to before the table is allowed to stick
/// out past the right edge.
const MIN_COLUMN: usize = 6;

/// Lays out `rows` of cells (each cell a run of spans). The first
/// `header_rows` rows are the header and get a line under them.
pub fn render(
    rows: &[Vec<Vec<Span>>],
    header_rows: usize,
    aligns: &[Align],
    width: usize,
    border: Style,
    chars: &BoxChars,
) -> Vec<Line> {
    let columns = rows
        .iter()
        .map(Vec::len)
        .chain([aligns.len()])
        .max()
        .unwrap_or(0);
    if columns == 0 {
        return Vec::new();
    }

    let natural: Vec<usize> = (0..columns)
        .map(|c| {
            rows.iter()
                .map(|row| natural_width(cell(row, c)))
                .max()
                .unwrap_or(0)
        })
        .collect();
    // Each column costs its content plus a space on either side and a border.
    let overhead = 3 * columns + 1;
    let widths = fit(&natural, width.saturating_sub(overhead));

    let laid_out: Vec<Vec<Vec<Line>>> = rows
        .iter()
        .map(|row| {
            (0..columns)
                .map(|c| wrap(cell(row, c), widths[c]))
                .collect()
        })
        .collect();
    let multiline_body = laid_out
        .iter()
        .skip(header_rows)
        .any(|row| row.iter().any(|lines| lines.len() > 1));

    let rule = |[left, mid, right]: [char; 3]| {
        let mut s = String::new();
        s.push(left);
        for (c, w) in widths.iter().enumerate() {
            if c > 0 {
                s.push(mid);
            }
            s.extend(std::iter::repeat_n(chars.horizontal, w + 2));
        }
        s.push(right);
        Line::plain(s, border)
    };

    let mut out = vec![rule(chars.top)];
    for (r, row) in laid_out.into_iter().enumerate() {
        if r > 0 && (r == header_rows || (r > header_rows && multiline_body)) {
            out.push(rule(chars.middle));
        }
        let height = row.iter().map(Vec::len).max().unwrap_or(1);
        for i in 0..height {
            let mut line = Line::new();
            for (c, lines) in row.iter().enumerate() {
                line.push_str(&format!("{} ", chars.vertical), border);
                let content = lines.get(i).cloned().unwrap_or_default();
                let gap = widths[c].saturating_sub(content.width());
                let align = aligns.get(c).copied().unwrap_or(Align::Left);
                let (before, after) = match align {
                    Align::Left => (0, gap),
                    Align::Right => (gap, 0),
                    Align::Center => (gap / 2, gap - gap / 2),
                };
                line.push_str(&" ".repeat(before), Style::PLAIN);
                line.append(content);
                line.push_str(&" ".repeat(after + 1), Style::PLAIN);
            }
            line.push_str(&chars.vertical.to_string(), border);
            out.push(line);
        }
    }
    out.push(rule(chars.bottom));
    out
}

/// A row's cell in column `c`; rows may be shorter than the header.
fn cell(row: &[Vec<Span>], c: usize) -> &[Span] {
    row.get(c).map_or(&[], Vec::as_slice)
}

/// Column widths: the natural ones if they fit in `available`, otherwise
/// the widest columns are narrowed first, one column at a time, so long
/// cells give way before short ones.
pub fn fit(natural: &[usize], available: usize) -> Vec<usize> {
    let mut widths: Vec<usize> = natural.iter().map(|&w| w.max(1)).collect();
    let floor = |c: usize| natural[c].clamp(1, MIN_COLUMN);
    let mut total: usize = widths.iter().sum();
    while total > available {
        let widest = (0..widths.len())
            .filter(|&c| widths[c] > floor(c))
            .max_by_key(|&c| (widths[c], std::cmp::Reverse(c)));
        let Some(c) = widest else { break };
        widths[c] -= 1;
        total -= 1;
    }
    widths
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::GLYPHS;

    fn cells(texts: &[&str]) -> Vec<Vec<Span>> {
        texts
            .iter()
            .map(|t| vec![Span::new(*t, Style::PLAIN)])
            .collect()
    }

    fn render_plain(rows: &[Vec<Vec<Span>>], aligns: &[Align], width: usize) -> Vec<String> {
        render(rows, 1, aligns, width, Style::PLAIN, &GLYPHS.table)
            .iter()
            .map(|l| crate::text::plain_text(&l.spans))
            .collect()
    }

    #[test]
    fn natural_widths_when_they_fit() {
        assert_eq!(fit(&[3, 10, 5], 40), [3, 10, 5]);
    }

    #[test]
    fn widest_column_gives_way_first() {
        assert_eq!(fit(&[4, 30, 10], 30), [4, 16, 10]);
        assert_eq!(fit(&[20, 20], 30), [15, 15]);
    }

    #[test]
    fn columns_stop_at_the_floor() {
        assert_eq!(fit(&[10, 10, 10], 5), [6, 6, 6]);
        assert_eq!(fit(&[2, 40], 4), [2, 6]);
    }

    #[test]
    fn alignment_and_borders() {
        let rows = vec![cells(&["a", "b", "c"]), cells(&["left", "mid", "right"])];
        let lines = render_plain(&rows, &[Align::Left, Align::Center, Align::Right], 80);
        assert_eq!(
            lines,
            [
                "┌──────┬─────┬───────┐",
                "│ a    │  b  │     c │",
                "├──────┼─────┼───────┤",
                "│ left │ mid │ right │",
                "└──────┴─────┴───────┘",
            ]
        );
    }

    #[test]
    fn long_cells_wrap_and_rows_get_separators() {
        let rows = vec![
            cells(&["k", "v"]),
            cells(&["one", "a b c d e f"]),
            cells(&["two", "x"]),
        ];
        let lines = render_plain(&rows, &[Align::Left, Align::Left], 18);
        assert_eq!(
            lines,
            [
                "┌─────┬──────────┐",
                "│ k   │ v        │",
                "├─────┼──────────┤",
                "│ one │ a b c d  │",
                "│     │ e f      │",
                "├─────┼──────────┤",
                "│ two │ x        │",
                "└─────┴──────────┘",
            ]
        );
    }
}
