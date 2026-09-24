.PHONY: update format line-count lint tests msrv coverage coverage_report build release install uninstall docs clean demo publish-demo

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
	cargo clippy --all-targets -- -D warnings
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# Tests share process-wide env vars and a cache file; run them single-threaded.
tests:
	RUST_LOG=debug RUST_BACKTRACE=1 cargo test -- --test-threads=1

msrv:
	cargo +$$(sed -n 's/^rust-version = "\(.*\)"/\1/p' Cargo.toml) check --all-targets --locked

coverage:
	@mkdir -p target/coverage
	cargo llvm-cov --lcov --output-path target/coverage/tests.lcov -- --test-threads=1

coverage_report:
	cargo llvm-cov --html --open -- --test-threads=1


build:
	cargo build

release:
	cargo build --release

install:
	cargo install --path .

uninstall:
	cargo uninstall

docs:
	cargo doc --workspace --no-deps --open

CHANGELOG.md:
	git cliff > CHANGELOG.md

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
