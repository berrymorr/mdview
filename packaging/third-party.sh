#!/bin/sh
# Prints THIRD-PARTY: the licences of everything in the mdview binary that
# is not mdview's own code. Takes the binary, which lists the licences of
# the syntaxes and themes built into it. Runs from the top of the source
# tree, after the build.
set -eu
bin=$1
rule='================================================================================'
section() {
    printf '\n%s\n%s\n%s\n\n' "$rule" "$1" "$rule"
}

cat <<'EOF'
mdview is free software under the GNU Affero General Public License,
version 3 or later; its text is in LICENSE. The program also holds the work
of others, each under its own licence, and this file gives them all:

1. The Rust crates mdview is built from.
2. Oniguruma, the C library of regular expressions inside the onig_sys
   crate, which the syntax highlighting runs on.
3. The syntax definitions and themes for code blocks. They are bat's
   (https://github.com/sharkdp/bat), built in by the two-face crate.
4. The GNU General Public License, version 3: the GNU Lesser General Public
   License of the ansi_colours crate is that licence with permissions
   added, and asks for both texts to come with the program.
EOF

section '1. The Rust crates'
# cargo-about reads every crate in Cargo.lock, so the ones for other
# systems are fetched too; then --frozen takes the licence texts the crates
# carry and nothing from the internet.
cargo fetch --locked --quiet
cargo about generate --frozen --config packaging/about.toml packaging/about.hbs

section '2. Oniguruma, inside the onig_sys crate'
onig=$(sed -n '/^name = "onig_sys"$/{n;s/^version = "\(.*\)"$/\1/p;}' Cargo.lock)
copying=$(ls "${CARGO_HOME:-$HOME/.cargo}"/registry/src/*/"onig_sys-$onig"/oniguruma/COPYING)
cat "$copying"

section '3. Syntax definitions and themes for code blocks'
"$bin" --generate acknowledgements

section '4. The GNU General Public License, version 3'
cat packaging/GPL-3.0
