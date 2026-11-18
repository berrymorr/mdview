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
the exact shades.

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
| `--hyperlinks` | make links clickable (OSC 8), for terminals that support it. |
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
