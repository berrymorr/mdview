# mdview

A terminal viewer for Markdown. Instead of the raw source it shows the
document laid out: headings, emphasis, lists, tables, block quotes, GitHub
alerts, footnotes, and code blocks highlighted with bat's syntax definitions
and themes. Paging and reading standard input work as in `bat`.

```sh
mdview README.md                  # one file, paged if it does not fit the screen
mdview docs/*.md                  # several, each under its name
curl -s https://example.com/README.md | mdview
mdview -P notes.md                # no pager
mdview --color=always notes.md | less -R
```

## What it shows

The dialect is GitHub's (GFM): tables, task lists, strikethrough, bare URLs
as links, footnotes, alerts (`> [!NOTE]`), `$math$`, front matter, plus
description lists. Paragraphs are re-wrapped to the width of the terminal.

Markup the terminal cannot draw becomes colour: *italic* text, for one, is
green rather than slanted (`--italic` adds the slant for terminals that can
draw it). Colours are the terminal's own 16, so its colour scheme decides
the exact shades, and each of them can be changed (see [Colours](#colours)).

| Markdown | Shown as |
|---|---|
| `# Heading 1` | bold bright magenta, a heavy line under it across the width |
| `## Heading 2` | bold bright blue, a thin line under it |
| `###`…`######` | bold: bright cyan, bright green, bright yellow, default colour |
| `**bold**` | bold |
| `*italic*` | green |
| `~~struck~~` | dim and struck through |
| `` `code` `` | yellow |
| `[text](url)` | bright blue, underlined; the address after it, dim, in brackets — unless the text is the address or it points inside the document (`#…`) |
| `![alt](src)` | `[image: alt]` in magenta and the path, dim |
| `> quote` | a dim `│` bar on the left |
| `> [!NOTE]` and the other alerts | a coloured `┃` bar and the alert's name: Note blue, Tip green, Important magenta, Warning yellow, Caution red |
| lists | `•` `◦` `▪` by depth, numbers right-aligned, markers yellow |
| `- [ ]` / `- [x]` | `☐` dim / `☑` green |
| code blocks | a dim frame with the language in its top line, the code highlighted |
| tables | dim box-drawing borders, header bold, columns aligned as written, cells wrapped to fit |
| `---` | a dim line across the width |
| footnotes | `[1]` in cyan; the notes themselves at the end, under a short line |
| `$math$` | magenta (the TeX source, unrendered) |
| HTML | formatting tags (`<b>`, `<kbd>`, `<br>`, `<img>`, `<a>`, `<details>`…) take effect, comments disappear, unknown tags are shown dim |

When the output is not a terminal, colours are off and the text comes out
laid out but plain — still at the terminal's width if there is one on
standard error, otherwise at `COLUMNS` or 80 columns.

## Options

| Option | |
|---|---|
| `--paging auto\|always\|never`, `-P` | `auto` (the default) pages when the output is a terminal; the pager quits by itself if the text fits on one screen. `-P` is `--paging=never`. |
| `--pager COMMAND` | the pager; before it `MDVIEW_PAGER`, then `PAGER`, then `less`. less is started with `-R -F` as bat does. |
| `--color auto\|always\|never` | `auto` colours only a terminal, and not when `NO_COLOR` is set. |
| `-w`, `--width COLUMNS` | lay the text out in this many columns instead of the terminal's width. |
| `--theme NAME`, `--list-themes` | syntax theme for code blocks; by default `MDVIEW_THEME`, then `BAT_THEME`, then `Monokai Extended`. |
| `--colors ELEMENT=STYLE`, `--list-colors` | the look of one part of the document, such as `emph=cyan`; list them all (see [Colours](#colours)). |
| `--hyperlinks` | make links clickable (OSC 8), for terminals that support it; one that does not may print the link codes as text. |
| `--italic` | draw emphasis slanted as well as coloured. |
| `--no-config` | ignore the configuration file. |

## Configuration

`~/.config/mdview/config` (or `$XDG_CONFIG_HOME/mdview/config`, or the file
named by `MDVIEW_CONFIG_PATH`) holds default arguments, as bat's config does:
one or more per line, `#` starts a comment. The command line wins over it.

```sh
# ~/.config/mdview/config
--theme="Monokai Extended Light"
--hyperlinks
```

## Colours

Any part of the document can be given its own look with
`--colors ELEMENT=STYLE`, repeated for as many parts as needed. The
configuration file is the place for it:

```sh
# ~/.config/mdview/config
--colors emph=cyan
--colors 'h1=bold brightred'
--colors 'mark=black brightyellow'
--colors code-frame=normal         # plain instead of dim
```

A style is written the way git writes colours in its configuration: words
in any order, separated by spaces or commas.

- Attributes: `bold`, `dim`, `italic`, `ul` (underline), `reverse`,
  `strike`.
- Colours: the first is the text's, the second the background's. A colour is
  `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan` or `white`;
  the same with `bright` in front, as in `brightred`; a number from 0 to 255
  (0–15 are those sixteen, the rest the 256-colour palette); `#rrggbb`; or
  `normal`, the terminal's own colour, so that `normal blue` sets only the
  background.

A style replaces the element's whole look: `h1=red` is red and no longer
bold. `normal` alone, or nothing after the `=`, leaves the element plain.
A `#rrggbb` colour goes to the terminal as it is only when `COLORTERM` is
`truecolor` or `24bit`; otherwise it becomes the nearest of the 256 palette
colours, as colours in code blocks do — a terminal without 24-bit colour
would misread it.

`mdview --list-colors` prints every element as `ELEMENT=STYLE`, each line
in its own style and with the configuration applied: it shows how the
styles come out in your terminal, and the lines can be copied into the
configuration file.

| Element | Default | What it colours |
|---|---|---|
| `h1` | `bold brightmagenta` | level 1 headings |
| `h1-rule` | `magenta` | the line under them |
| `h2` | `bold brightblue` | level 2 headings |
| `h2-rule` | `blue` | the line under them |
| `h3` | `bold brightcyan` | level 3 headings |
| `h4` | `bold brightgreen` | level 4 headings |
| `h5` | `bold brightyellow` | level 5 headings |
| `h6` | `bold` | level 6 headings |
| `strong` | `bold` | `**bold**` |
| `emph` | `green` | `*italic*` |
| `strike` | `dim strike` | `~~struck~~` |
| `code` | `yellow` | `` `code` `` |
| `mark` | `black yellow` | `<mark>` |
| `kbd` | `reverse` | `<kbd>` |
| `link` | `ul brightblue` | the text of a link |
| `url` | `dim` | the address after a link or an image |
| `image` | `magenta` | `[image: …]` |
| `footnote` | `cyan` | footnote references, `[1]` |
| `math` | `magenta` | `$math$` and math blocks |
| `html` | `dim` | HTML tags shown as they are |
| `quote-bar` | `dim` | the bar left of a block quote |
| `quote-text` | `normal` | the text in a block quote |
| `list-marker` | `yellow` | bullets and numbers |
| `task-open` | `dim` | `☐` |
| `task-done` | `green` | `☑` |
| `rule` | `dim` | `---`, and the line above the footnotes |
| `table-border` | `dim` | the lines of a table |
| `table-header` | `bold` | its header row |
| `code-frame` | `dim` | the frame around a code block |
| `code-label` | `dim` | the language named in the frame |
| `code-text` | `normal` | code that is not highlighted |
| `term` | `bold` | the terms of a description list |
| `file-header` | `bold reverse` | the name above each file when there are several |
| `note` | `brightblue` | `> [!NOTE]`: the bar and the title |
| `tip` | `brightgreen` | `> [!TIP]` |
| `important` | `brightmagenta` | `> [!IMPORTANT]` |
| `warning` | `brightyellow` | `> [!WARNING]` |
| `caution` | `brightred` | `> [!CAUTION]` |

## Building

Everything runs in Docker; the host needs nothing else.

```sh
make test       # unit and output tests
make lint       # rustfmt and clippy
make build      # target/release/mdview
make install    # build, then copy to ~/.local/bin (PREFIX=… to change)
```

`make` uses `sudo -n docker`; set `DOCKER=docker` if your user may run it
directly. The output tests compare against files in `tests/golden/`;
`UPDATE_GOLDEN=1` rewrites them after an intended change.
