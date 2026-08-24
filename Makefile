.PHONY: help fmt fmt-check lint test build build-release check install

CARGO ?= cargo
INSTALL ?= /usr/bin/install
INSTALL_DIR ?= /usr/local/bin
BINARY := target/release/pm

help:
	@echo "Targets:"
	@echo "  fmt           - format all Rust code"
	@echo "  fmt-check     - verify Rust formatting"
	@echo "  lint          - run Clippy with warnings as errors"
	@echo "  test          - run all tests"
	@echo "  build         - build all development targets"
	@echo "  build-release - build the optimized pm binary"
	@echo "  check         - run fmt-check, lint, and test"
	@echo "  install       - build and install pm (default: /usr/local/bin)"

fmt:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt --all --check

lint:
	$(CARGO) clippy --all-targets --locked -- -D warnings

test:
	$(CARGO) test --locked

build:
	$(CARGO) build --all-targets --locked

build-release:
	$(CARGO) build --release --locked

check: fmt-check lint test

install: build-release
	$(INSTALL) -d "$(INSTALL_DIR)"
	$(INSTALL) -v -m 0755 "$(BINARY)" "$(INSTALL_DIR)/pm"
	"$(INSTALL_DIR)/pm" --version
