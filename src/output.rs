//! Where the rendered text goes: straight to the terminal, or through a
//! pager — chosen and started the way bat does it.

use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Less,
    More,
    Most,
    Mdview,
    Other,
}

impl Kind {
    fn of(bin: &str) -> Kind {
        match Path::new(bin).file_stem().and_then(|s| s.to_str()) {
            Some("less") => Kind::Less,
            Some("more") => Kind::More,
            Some("most") => Kind::Most,
            Some("mdview") => Kind::Mdview,
            _ => Kind::Other,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Pager {
    pub bin: String,
    pub args: Vec<String>,
    pub kind: Kind,
    /// Came from the generic `PAGER` variable rather than from something
    /// meant for mdview.
    pub generic: bool,
}

/// The pager to run: `--pager`, else `MDVIEW_PAGER`, else `PAGER`, else
/// `less`. An empty command means no pager at all.
pub fn choose(
    flag: Option<&str>,
    mdview_pager: Option<&str>,
    pager: Option<&str>,
) -> Result<Option<Pager>, String> {
    let (command, generic) = match (flag, mdview_pager, pager) {
        (Some(cmd), _, _) => (cmd, false),
        (None, Some(cmd), _) => (cmd, false),
        (None, None, Some(cmd)) => (cmd, true),
        (None, None, None) => ("less", false),
    };
    let parts = shell_words::split(command)
        .map_err(|e| format!("cannot parse the pager command '{command}': {e}"))?;
    let Some((bin, args)) = parts.split_first() else {
        return Ok(None);
    };
    let kind = Kind::of(bin);
    // `more` and `most` cannot show colours, and mdview paging itself would
    // never end. Only the generic PAGER is overridden for that: a pager
    // named for mdview on purpose is used as given.
    if generic && matches!(kind, Kind::More | Kind::Most | Kind::Mdview) {
        return Ok(Some(Pager {
            bin: "less".to_string(),
            args: Vec::new(),
            kind: Kind::Less,
            generic,
        }));
    }
    Ok(Some(Pager {
        bin: bin.clone(),
        args: args.to_vec(),
        kind,
        generic,
    }))
}

impl Pager {
    /// The arguments to start the pager with. less needs `-R` to pass colours
    /// through, so when it runs with no arguments of its own — or with ones
    /// from the generic PAGER, which were not written with colour in mind —
    /// it gets bat's set instead.
    pub fn arguments(
        &self,
        quit_if_one_screen: bool,
        less_version: impl FnOnce() -> Option<u32>,
    ) -> Vec<String> {
        if self.kind != Kind::Less || !(self.args.is_empty() || self.generic) {
            return self.args.clone();
        }
        let mut args = vec!["-R".to_string()];
        if quit_if_one_screen {
            args.push("-F".to_string());
        }
        // Before less 530, -F without --no-init cleared the screen of a short
        // text the moment less quit. --no-init costs mouse-wheel scrolling,
        // so it is only added for those versions (or an unknown one).
        if !less_version().is_some_and(|v| v >= 530) {
            args.push("--no-init".to_string());
        }
        args
    }
}

fn less_version(bin: &str) -> Option<u32> {
    let out = Command::new(bin)
        .arg("--version")
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let first = text.lines().next()?;
    first
        .strip_prefix("less ")?
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

/// Writes `text` to the terminal, through the pager when `paging` is on.
/// Quitting the pager early is not an error.
pub fn emit(
    text: &str,
    paging: bool,
    quit_if_one_screen: bool,
    pager_flag: Option<&str>,
) -> Result<(), String> {
    if paging {
        let mdview_pager = std::env::var("MDVIEW_PAGER").ok();
        let pager = std::env::var("PAGER").ok();
        if let Some(pager) = choose(pager_flag, mdview_pager.as_deref(), pager.as_deref())? {
            let args = pager.arguments(quit_if_one_screen, || less_version(&pager.bin));
            let mut command = Command::new(&pager.bin);
            command.args(&args).stdin(Stdio::piped());
            if pager.kind == Kind::Less {
                command.env("LESSCHARSET", "UTF-8");
            }
            // A pager that cannot be started is no reason to show nothing.
            if let Ok(mut child) = command.spawn() {
                ignore_interrupts();
                let written = child
                    .stdin
                    .take()
                    .map(|mut stdin| stdin.write_all(text.as_bytes()));
                let waited = child.wait();
                if let Some(Err(e)) = written
                    && e.kind() != io::ErrorKind::BrokenPipe
                {
                    return Err(format!("writing to the pager: {e}"));
                }
                waited.map_err(|e| format!("waiting for the pager: {e}"))?;
                return Ok(());
            }
        }
    }
    let mut stdout = io::stdout().lock();
    match stdout
        .write_all(text.as_bytes())
        .and_then(|()| stdout.flush())
    {
        Err(e) if e.kind() != io::ErrorKind::BrokenPipe => Err(format!("writing the output: {e}")),
        _ => Ok(()),
    }
}

/// Lets Ctrl-C reach the pager alone. The pager uses it to cancel a search;
/// if mdview died of it instead, the shell would take the terminal back
/// while the pager is still drawing on it. git does the same.
fn ignore_interrupts() {
    // SAFETY: setting a signal to SIG_IGN installs no handler, and nothing
    // else in this process deals with signals.
    unsafe {
        libc::signal(libc::SIGINT, libc::SIG_IGN);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pager(flag: Option<&str>, own: Option<&str>, generic: Option<&str>) -> Pager {
        choose(flag, own, generic).unwrap().unwrap()
    }

    #[test]
    fn precedence() {
        assert_eq!(pager(Some("a"), Some("b"), Some("c")).bin, "a");
        assert_eq!(pager(None, Some("b"), Some("c")).bin, "b");
        assert_eq!(pager(None, None, Some("c")).bin, "c");
        assert_eq!(pager(None, None, None).bin, "less");
        assert_eq!(choose(None, Some(""), None).unwrap(), None);
    }

    #[test]
    fn colourless_pagers_from_pager_become_less() {
        assert_eq!(pager(None, None, Some("more")).bin, "less");
        assert_eq!(pager(None, None, Some("/usr/bin/most -s")).bin, "less");
        assert_eq!(pager(None, Some("more"), None).bin, "more");
    }

    #[test]
    fn less_gets_colour_arguments() {
        let new_less = || Some(590);
        assert_eq!(
            pager(None, None, None).arguments(true, new_less),
            ["-R", "-F"]
        );
        assert_eq!(pager(None, None, None).arguments(false, new_less), ["-R"]);
        assert_eq!(
            pager(None, None, None).arguments(true, || Some(487)),
            ["-R", "-F", "--no-init"]
        );
        // PAGER="less -i" was not written with colours in mind; replaced.
        assert_eq!(
            pager(None, None, Some("less -i")).arguments(true, new_less),
            ["-R", "-F"]
        );
        // MDVIEW_PAGER and --pager are taken as they are.
        assert_eq!(
            pager(None, Some("less -S"), None).arguments(true, new_less),
            ["-S"]
        );
        assert_eq!(
            pager(Some("cat"), None, None).arguments(true, new_less),
            Vec::<String>::new()
        );
    }
}
