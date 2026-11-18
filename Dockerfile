# Build and test environment. The host needs nothing but Docker: `make`
# runs cargo in a container from this image and removes it afterwards.
# rust:slim-bookworm with Rust 1.100.0, pinned by digest.
FROM rust:slim-bookworm@sha256:7c7d4eb4d05b7781187a327809466cfddf29753f6defdc3d19a0e291951b7a37
RUN rustup component add clippy rustfmt
