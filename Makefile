# Every target runs cargo in a throw-away container built from ./Dockerfile.
# The cargo cache lives in target/cargo-home, so `rm -rf target` removes all
# of it.

DOCKER ?= sudo -n docker
IMAGE  := mdview-build
PREFIX ?= $(HOME)/.local
RUN     = $(DOCKER) run --rm --user $$(id -u):$$(id -g) -v "$(CURDIR)":/work -w /work \
          -e CARGO_HOME=/work/target/cargo-home -e TZ=UTC $(IMAGE)

.PHONY: image build test lint fmt install remove-image

image:
	$(DOCKER) build -q -t $(IMAGE) . >/dev/null

build: image
	$(RUN) cargo build --release --locked

test: image
	$(RUN) cargo test --locked

lint: image
	$(RUN) sh -c 'cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings'

fmt: image
	$(RUN) cargo fmt

install: build
	install -D -m 755 target/release/mdview $(PREFIX)/bin/mdview

remove-image:
	$(DOCKER) image rm $(IMAGE)
