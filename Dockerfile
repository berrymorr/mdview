# Two images from one base: Rust 1.100.0 on Alpine 3.24, pinned by digest.
# Its Rust targets musl, so whatever it builds is a static binary.
#
# dev: the toolchain. `make` runs cargo in a throw-away container from it, so
# the host needs nothing but Docker.
#
# packages: the release files for the machine's own architecture, the tests
# run first. `make package` builds it into dist/, and so do the GitLab jobs
# on amd64 and arm64. packaging/build.sh says what is in it.
FROM docker.io/library/rust:1.100.0-alpine3.24@sha256:8d0e82abfdf9a38071e07cabc5afba4ddbe077631a80e4e742a760c38beadc33 AS dev
RUN apk add --no-cache build-base && rustup component add clippy rustfmt

FROM dev AS build
RUN apk add --no-cache curl tar xz
# nfpm makes the .deb and .rpm, cargo-about lists the crates' licences:
# their release binaries, checked against the checksums published with them.
ARG NFPM_VERSION=2.48.0
ARG CARGO_ABOUT_VERSION=0.9.3
RUN set -eu; \
    case $(uname -m) in \
      x86_64) nfpm=x86_64 nfpm_sum=6d7f9a28b2f397934403950fcbba83f598b08d02edbe79a0815b3b4897341750 \
              about_sum=3ad1e94ea1e63d69d1961f4fb15ccbbe9575d906111f96d2cb4dce6df48f71b4 ;; \
      aarch64) nfpm=arm64 nfpm_sum=a9a75213763783b7669719e505474c748c5ad7dc66591a195619deb50f85c2f4 \
               about_sum=eb4d9ff6ebd2518152a8308729431a04b3836e71d2bd5ead7aea69f91a71162e ;; \
    esac; \
    about=cargo-about-$CARGO_ABOUT_VERSION-$(uname -m)-unknown-linux-musl; \
    cd /tmp; \
    curl -fsSL -o nfpm.tar.gz "https://github.com/goreleaser/nfpm/releases/download/v$NFPM_VERSION/nfpm_${NFPM_VERSION}_Linux_$nfpm.tar.gz"; \
    curl -fsSL -o about.tar.gz "https://github.com/EmbarkStudios/cargo-about/releases/download/$CARGO_ABOUT_VERSION/$about.tar.gz"; \
    printf '%s  nfpm.tar.gz\n%s  about.tar.gz\n' "$nfpm_sum" "$about_sum" | sha256sum -c -; \
    tar -xzf nfpm.tar.gz -C /usr/local/bin nfpm; \
    tar -xzf about.tar.gz -C /usr/local/bin --strip-components=1 "$about/cargo-about"; \
    rm nfpm.tar.gz about.tar.gz
WORKDIR /src
COPY . .
# The time the files in the packages carry: the commit's, given by the caller.
ARG SOURCE_DATE_EPOCH
# How many compile jobs cargo runs; unset, one per CPU it can see. A build
# whose CPUs are capped by a quota sees them all, so it says how many.
ARG JOBS
RUN packaging/build.sh /out

FROM scratch AS packages
COPY --from=build /out/ /
