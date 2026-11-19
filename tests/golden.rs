//! Renders Markdown files and compares the output with the expected output
//! kept in tests/golden/. `UPDATE_GOLDEN=1 cargo test` rewrites the
//! expectations instead; review the diff before committing it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// mdview, with nothing from the environment that would change its output.
fn mdview() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mdview"));
    for var in [
        "MDVIEW_THEME",
        "BAT_THEME",
        "COLORTERM",
        "MDVIEW_PAGER",
        "NO_COLOR",
        "MDVIEW_CONFIG_PATH",
    ] {
        command.env_remove(var);
    }
    command
}

fn run_command(mut command: Command, stdin: Option<&str>) -> Output {
    use std::io::Write;
    let mut child = command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("mdview runs");
    let mut input = child.stdin.take().unwrap();
    if let Some(text) = stdin {
        input.write_all(text.as_bytes()).unwrap();
    }
    drop(input);
    child.wait_with_output().unwrap()
}

fn run(args: &[&str], stdin: Option<&str>) -> Output {
    let mut command = mdview();
    command.arg("--no-config").args(args);
    run_command(command, stdin)
}

fn stdout(out: Output) -> String {
    String::from_utf8(out.stdout).unwrap()
}

fn render(args: &[&str], file: &Path) -> String {
    let file = file.to_str().unwrap();
    let mut all = vec!["--paging=never"];
    all.extend_from_slice(args);
    all.push(file);
    let out = run(&all, None);
    assert!(
        out.status.success(),
        "mdview failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

fn check(expected_file: &str, actual: &str) {
    let path = root().join("tests/golden").join(expected_file);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    if expected != actual {
        let first = expected
            .lines()
            .zip(actual.lines())
            .position(|(e, a)| e != a)
            .unwrap_or_else(|| expected.lines().count().min(actual.lines().count()));
        panic!(
            "{expected_file} differs from line {}:\nexpected: {:?}\n  actual: {:?}\n(UPDATE_GOLDEN=1 rewrites it)",
            first + 1,
            expected.lines().nth(first).unwrap_or("<end>"),
            actual.lines().nth(first).unwrap_or("<end>"),
        );
    }
}

fn showcase() -> PathBuf {
    root().join("examples/showcase.md")
}

#[test]
fn showcase_plain() {
    check(
        "showcase.txt",
        &render(&["--color=never", "--width=72"], &showcase()),
    );
}

#[test]
fn showcase_styled() {
    check(
        "showcase.ansi",
        &render(&["--color=always", "--width=72"], &showcase()),
    );
}

#[test]
fn showcase_narrow() {
    check(
        "showcase-narrow.txt",
        &render(&["--color=never", "--width=30"], &showcase()),
    );
}

#[test]
fn edge_cases() {
    let file = root().join("tests/golden/edge.md");
    check("edge.txt", &render(&["--color=never", "--width=40"], &file));
}

#[test]
fn hyperlinks() {
    let out = run(
        &[
            "--paging=never",
            "--color=always",
            "--hyperlinks",
            "-w",
            "40",
        ],
        Some("[a](https://x.io) [b](rel.md)\n"),
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("\x1b]8;;https://x.io\x1b\\"), "{text:?}");
    assert!(
        !text.contains("rel.md\x1b\\"),
        "relative paths are not linked: {text:?}"
    );
}

#[test]
fn reads_standard_input() {
    let out = run(&["--color=never", "-w", "20"], Some("Some *text*.\n"));
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "Some text.\n");
    let dash = run(&["--color=never", "-w", "20", "-"], Some("# Title\n"));
    assert_eq!(
        String::from_utf8(dash.stdout).unwrap(),
        "Title\n━━━━━━━━━━━━━━━━━━━━\n"
    );
}

#[test]
fn missing_files_are_reported_and_the_rest_shown() {
    let good = showcase();
    let out = run(
        &[
            "--color=never",
            "-w",
            "40",
            "no-such-file.md",
            good.to_str().unwrap(),
        ],
        None,
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.starts_with("mdview: no-such-file.md: "), "{stderr}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Heading level 1"));
}

#[test]
fn several_files_get_headers() {
    let file = showcase();
    let path = file.to_str().unwrap();
    let out = run(&["--color=never", "-w", "40", path, path], None);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.matches("showcase.md").count(), 2);
}

#[test]
fn pager_receives_the_text() {
    let direct = run(
        &["--color=never", "-w", "30", "--paging=never"],
        Some("# A\n\ntext\n"),
    );
    let paged = run(
        &[
            "--color=never",
            "-w",
            "30",
            "--paging=always",
            "--pager=cat",
        ],
        Some("# A\n\ntext\n"),
    );
    assert_eq!(direct.stdout, paged.stdout);
}

#[test]
fn colors_change_the_look_of_elements() {
    let out = run(
        &[
            "--color=always",
            "-w",
            "12",
            "--colors=emph=cyan",
            "--colors",
            "h1=bold red",
            "--colors=h1-rule=",
        ],
        Some("# Title\n\n*it*\n"),
    );
    assert_eq!(
        stdout(out),
        "\x1b[0;1;31mTitle\x1b[0m\n━━━━━━━━━━━━\n\n\x1b[0;36mit\x1b[0m\n"
    );
}

#[test]
fn rgb_colors_fit_the_palette_unless_the_terminal_takes_them() {
    let args = ["--no-config", "--color=always", "--colors=code=#ff8700"];
    let mut command = mdview();
    command.args(args);
    let fitted = stdout(run_command(command, Some("`x`\n")));
    assert!(fitted.contains("\x1b[0;38;5;208mx"), "{fitted:?}");
    let mut command = mdview();
    command.args(args).env("COLORTERM", "truecolor");
    let rgb = stdout(run_command(command, Some("`x`\n")));
    assert!(rgb.contains("\x1b[0;38;2;255;135;0mx"), "{rgb:?}");
}

#[test]
fn colors_come_from_the_config_file_and_the_command_line_wins() {
    let config = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("colors-config");
    std::fs::write(&config, "--colors='emph=cyan'\n--colors 'code=red'\n").unwrap();
    let mut command = mdview();
    command.env("MDVIEW_CONFIG_PATH", &config).args([
        "--color=always",
        "-w",
        "20",
        "--colors=code=green",
    ]);
    let text = stdout(run_command(command, Some("*a* `b`\n")));
    assert!(text.contains("\x1b[0;36ma"), "{text:?}");
    assert!(text.contains("\x1b[0;32mb"), "{text:?}");
}

#[test]
fn unknown_elements_and_styles_are_refused() {
    for (setting, complaint) in [
        ("h7=red", "there is no element 'h7'"),
        ("emph=blod", "'blod' is neither"),
        ("emph", "expected ELEMENT=STYLE"),
    ] {
        let out = run(&["--colors", setting], Some(""));
        assert_eq!(out.status.code(), Some(2), "{setting}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(stderr.contains(complaint), "{setting}: {stderr}");
    }
}

#[test]
fn list_colors_shows_each_element_in_its_style() {
    let plain = stdout(run(
        &[
            "--list-colors",
            "--color=never",
            "--colors=emph=cyan",
            "--colors=code=#ff8700",
            "--italic",
        ],
        None,
    ));
    assert!(
        plain.starts_with("h1=bold brightmagenta\nh1-rule=magenta\nh2=bold brightblue\n"),
        "{plain}"
    );
    assert!(plain.contains("\nemph=italic cyan\n"), "{plain}");
    assert!(plain.contains("\ncode=#ff8700\n"), "{plain}");
    assert!(plain.contains("\nquote-text=normal\n"), "{plain}");
    let styled = stdout(run(
        &["--list-colors", "--color=always", "--colors=code=#ff8700"],
        None,
    ));
    assert!(
        styled.starts_with("\x1b[0;1;95mh1=bold brightmagenta\x1b[0m\n"),
        "{styled:?}"
    );
    assert!(
        styled.contains("\x1b[0;38;5;208mcode=#ff8700\x1b[0m\n"),
        "{styled:?}"
    );
}

/// The README's table of elements is the default theme, word for word.
#[test]
fn readme_lists_every_element_with_its_default() {
    let readme = std::fs::read_to_string(root().join("README.md")).unwrap();
    let documented: Vec<String> = readme
        .lines()
        .filter_map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            let name = cells.get(1)?.strip_prefix('`')?.strip_suffix('`')?;
            let style = cells.get(2)?.strip_prefix('`')?.strip_suffix('`')?;
            Some(format!("{name}={style}"))
        })
        .collect();
    let listed = stdout(run(&["--list-colors", "--color=never"], None));
    assert_eq!(documented, listed.lines().collect::<Vec<_>>());
}

/// What packaging/build.sh puts in the packages beside the binary. Only
/// the packaging asks for these, so neither --help nor what it makes
/// mentions --generate.
#[test]
fn generate_prints_the_man_page_completions_and_acknowledgements() {
    for (what, expected) in [
        ("man", ".TH mdview 1"),
        ("bash", "complete -F _mdview"),
        ("zsh", "#compdef mdview"),
        ("fish", "complete -c mdview -l list-colors"),
        (
            "acknowledgements",
            "syntaxes/02_Extra/AWK/LICENSE\n\nThe MIT License",
        ),
    ] {
        let out = run(&["--generate", what], None);
        assert!(out.status.success(), "{what}");
        let text = stdout(out);
        assert!(text.contains(expected), "{what}: {text}");
        if what != "acknowledgements" {
            assert!(!text.contains("generate"), "{what}: {text}");
        }
    }
    assert!(!stdout(run(&["--help"], None)).contains("generate"));
}
