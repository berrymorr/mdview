---
title: mdview showcase
tags: [markdown, terminal]
---

# Heading level 1

Paragraph text with **bold**, *italic*, ***bold italic***, ~~strikethrough~~,
`inline code` and a [link](https://example.com/docs "Example"). Bare
addresses become links too: https://github.com/kivikakk/comrak and
mail@example.com. A footnote reference[^note] and inline math
$e^{i\pi} + 1 = 0$.

## Heading level 2

### Heading level 3

#### Heading level 4

##### Heading level 5

###### Heading level 6

Setext heading
--------------

## Lists

- First item
- Second item, long enough to wrap around at the edge of the terminal so that
  the hanging indent shows
  - Nested item
    - Deeper item
- Third item

1. One
2. Two
   1. Two point one
3. Three

- [ ] An open task
- [x] A finished task

A loose list:

1. First paragraph.

   Second paragraph of the same item.

2. Next item.

## Quotes and alerts

> A quote with *emphasis* and `code`.
>
> > A nested quote.

> [!NOTE]
> Useful information.

> [!TIP]
> Helpful advice.

> [!IMPORTANT]
> Key information.

> [!WARNING]
> Urgent information that needs attention.

> [!CAUTION]
> Advice about risks.

## Code

```rust
fn main() {
    let name = "world";
    println!("Hello, {name}!"); // a comment
}
```

    an indented code block
    has no language

```
a fenced block without a language
```

## Table

| Left | Center | Right |
|:-----|:------:|------:|
| a | b | c |
| `code` | **bold** | *italic* |

## Other markup

Term
: The definition of the term.

A line with a hard break\
continues here.

Keys: <kbd>Ctrl</kbd>+<kbd>C</kbd>, H<sub>2</sub>O, x<sup>2</sup>,
<mark>marked</mark>, <u>underlined</u>.

![A screenshot](docs/screenshot.png)

[![Build status](https://img.shields.io/badge/build-passing-green)](https://ci.example.com/mdview)

<!-- an HTML comment is not shown -->

<details>
<summary>Click to expand</summary>

Hidden content.

</details>

$$
\int_0^1 x^2\,dx = \frac{1}{3}
$$

Wide and non-Latin text: Привет, мир! 日本語のテキスト.

---

[^note]: The footnote text.
