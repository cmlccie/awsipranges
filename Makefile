.PHONY: update format lint test build release install uninstall clean demo

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
	cargo clippy

tests:
	RUST_BACKTRACE=1 cargo test

coverage: export CARGO_INCREMENTAL=0
coverage: export CARGO_ENCODED_RUSTFLAGS=-Cinstrument-coverage
coverage: export LLVM_PROFILE_FILE=cargo-test-%p-%m.profraw
coverage:
	@mkdir -p target/coverage
	@rm -rf target/coverage/*
	cargo test
	@grcov . --binary-path ./target/debug/deps/ -s . -t lcov --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o target/coverage/tests.lcov
	@find . -name '*.profraw' -delete

coverage_report: export CARGO_INCREMENTAL=0
coverage_report: export CARGO_ENCODED_RUSTFLAGS=-Cinstrument-coverage
coverage_report: export LLVM_PROFILE_FILE=cargo-test-%p-%m.profraw
coverage_report:
	@mkdir -p target/coverage
	@rm -rf target/coverage/*
	cargo test
	@grcov . --binary-path ./target/debug/deps/ -s . -t html --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o target/coverage/html
	@find . -name '*.profraw' -delete
	open target/coverage/html/index.html


build:
	cargo build

release:
	cargo build --release

install:
	cargo install --path .

uninstall:
	cargo uninstall

docs:
	cargo doc --open

clean:
	cargo clean
	rm -rf debug/
	rm -rf target/
	find . -name '*.rs.bk' -delete
	find . -name '*.pdb' -delete

demo:
	docker build -f demo/Dockerfile -t awsipranges-demo:local .
	docker run --rm -v $$(pwd)/demo:/vhs awsipranges-demo:local demo.tape

publish-demo:
	vhs publish demo/awsipranges.gif
