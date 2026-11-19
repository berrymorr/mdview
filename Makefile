# Every target runs in a throw-away container built from ./Dockerfile: cargo
# in its dev image, with the cargo cache in target/cargo-home, so
# `rm -rf target` removes all of it. `make package` builds its packages stage.

# docker itself when this user may run it, otherwise through sudo.
ifeq ($(origin DOCKER), undefined)
DOCKER := $(shell docker info >/dev/null 2>&1 && echo docker || echo sudo -n docker)
endif
IMAGE  := mdview-build
PREFIX ?= $(HOME)/.local
RUN     = $(DOCKER) run --rm --user $$(id -u):$$(id -g) -v "$(CURDIR)":/work -w /work \
          -e CARGO_HOME=/work/target/cargo-home -e TZ=UTC $(IMAGE)

.PHONY: image build test lint fmt install package remove-image

image:
	$(DOCKER) build -q --target dev -t $(IMAGE) . >/dev/null

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

# The .deb, .rpm and .tar.xz for this machine's architecture, in dist/. The
# build streams them out as a tar, so they belong to you even under sudo.
package:
	rm -rf dist && mkdir dist
	$(DOCKER) build --target packages \
	    --build-arg SOURCE_DATE_EPOCH=$$(git log -1 --format=%ct) -o - . | tar -x -C dist
	ls -l dist

remove-image:
	$(DOCKER) image rm $(IMAGE)
