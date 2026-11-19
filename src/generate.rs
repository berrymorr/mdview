//! What the packages carry besides the program: its man page, completions
//! for bash, zsh and fish, and the licences of the syntax definitions and
//! themes built into it. `--generate WHAT` prints one of them; the option is
//! hidden, being there for packaging/build.sh.

use std::io::{self, Write};

use clap::{Command, CommandFactory, ValueEnum};
use clap_complete::Shell;

use crate::cli::Args;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Generate {
    Man,
    Bash,
    Zsh,
    Fish,
    Acknowledgements,
}

pub fn print(what: Generate) -> Result<(), String> {
    let mut out = io::stdout().lock();
    let written = match what {
        Generate::Man => clap_mangen::Man::new(Args::command()).render(&mut out),
        Generate::Bash => complete(Shell::Bash, &mut out),
        Generate::Zsh => complete(Shell::Zsh, &mut out),
        Generate::Fish => complete(Shell::Fish, &mut out),
        Generate::Acknowledgements => acknowledgements(&mut out),
    };
    match written.and_then(|()| out.flush()) {
        Err(e) if e.kind() != io::ErrorKind::BrokenPipe => Err(format!("writing the output: {e}")),
        _ => Ok(()),
    }
}

/// Completions offer hidden options too, which suits --no-italic and
/// --no-hyperlinks but not --generate, so it is left out of the command
/// they are made from.
fn complete(shell: Shell, out: &mut impl Write) -> io::Result<()> {
    let options: Vec<_> = Args::command()
        .get_arguments()
        .filter(|arg| arg.get_id() != "generate")
        .cloned()
        .collect();
    let mut command = Command::new("mdview")
        .version(env!("CARGO_PKG_VERSION"))
        .args(options);
    clap_complete::generate(shell, &mut command, "mdview", out);
    Ok(())
}

/// The licence of each syntax definition and theme that asks to be named
/// where it is used, under the path of its file in bat's tree.
fn acknowledgements(out: &mut impl Write) -> io::Result<()> {
    let listing = two_face::acknowledgement::listing();
    let licenses = listing.for_syntaxes().iter().chain(listing.for_themes());
    for license in licenses.filter(|license| license.needs_acknowledgement()) {
        let text = license.text.trim_end();
        writeln!(out, "{}\n\n{text}\n", license.rel_path.display())?;
    }
    Ok(())
}
