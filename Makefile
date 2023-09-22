.PHONY: update format lint test build release install uninstall clean

.DEFAULT_GOAL := build

# -----------------------------------------------------------------------------
# Developer Operations
# -----------------------------------------------------------------------------

update:
	cargo update

format:
	cargo fmt

line-count:
	find src -name '*.rs' | xargs wc -l

lint:
	cargo fmt --check
	cargo check

test:
	cargo test

build:
	cargo build

release:
	cargo build --release

install:
	cargo install --path .

uninstall:
	cargo uninstall

clean:
	cargo clean
	rm -rf debug/
	rm -rf target/
	find . -name '*.rs.bk' -delete
	find . -name '*.pdb' -delete
