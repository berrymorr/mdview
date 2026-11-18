//! Renders Markdown files and compares the output with the expected output
//! kept in tests/golden/. `UPDATE_GOLDEN=1 cargo test` rewrites the
//! expectations instead; review the diff before committing it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run(args: &[&str], stdin: Option<&str>) -> Output {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_mdview"))
        .arg("--no-config")
        .args(args)
        .env_remove("MDVIEW_THEME")
        .env_remove("BAT_THEME")
        .env_remove("COLORTERM")
        .env_remove("MDVIEW_PAGER")
        .env_remove("NO_COLOR")
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
