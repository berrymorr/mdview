#!/bin/sh
# Builds the release files for this machine's architecture into the
# directory given: the tests first, then a static binary with its man page,
# completions and THIRD-PARTY, packed as .deb, .rpm and .tar.xz. It runs in
# the Dockerfile's packages stage, from the top of the source tree.
set -eu
out=$1
target=$(rustc -vV | sed -n 's/^host: //p')
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
case $target in
    x86_64-*) arch=amd64 ;;
    aarch64-*) arch=arm64 ;;
    *) echo "no packages for $target" >&2; exit 1 ;;
esac
: "${SOURCE_DATE_EPOCH:=$(date +%s)}"
export SOURCE_DATE_EPOCH
if [ -n "${JOBS:-}" ]; then
    export CARGO_BUILD_JOBS="$JOBS"
fi

cargo test --release --locked --target "$target"
cargo build --release --locked --target "$target"
bin=target/$target/release/mdview

# A static binary runs on any distribution, whatever its C library.
if readelf -lW "$bin" | grep -q 'program interpreter'; then
    echo "$bin needs a dynamic loader" >&2
    exit 1
fi

# The files go into target/package/files, which the .tar.xz names after the
# version and target, and nfpm.yaml takes as it is.
name=mdview-$version-$target
files=target/package/files
rm -rf target/package
mkdir -p "$files/complete"
cp "$bin" README.md LICENSE "$files/"
"$bin" --generate man > "$files/mdview.1"
"$bin" --generate bash > "$files/complete/mdview.bash"
"$bin" --generate zsh > "$files/complete/_mdview"
"$bin" --generate fish > "$files/complete/mdview.fish"
packaging/third-party.sh "$bin" > "$files/THIRD-PARTY"
gzip -9n < "$files/mdview.1" > target/package/mdview.1.gz

mkdir -p "$out"
tar --sort=name --owner=0 --group=0 --numeric-owner --mtime="@$SOURCE_DATE_EPOCH" \
    --transform="s|^files|$name|" -C target/package -cJf "$out/$name.tar.xz" files
# nfpm takes the paths in nfpm.yaml from the directory it is started in.
cd target/package
for packager in deb rpm; do
    VERSION=$version ARCH=$arch \
        nfpm package --config ../../packaging/nfpm.yaml --packager "$packager" --target "$out/"
done
ls -l "$out"
