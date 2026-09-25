# awsipranges

Rust CLI and library crate for searching and filtering the public AWS IP ranges
(`https://ip-ranges.amazonaws.com/ip-ranges.json`). Published to crates.io; binaries
are released through dist (Homebrew tap, shell/PowerShell installers, MSI).

## Commands

Use the Makefile targets; they mirror CI.

- `make lint` — `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
  rustdoc with `-D warnings`. CI fails on any warning.
- `make tests` — runs `cargo test` (tests are parallel-safe).
- `make msrv` — `cargo check` on the `rust-version` in `Cargo.toml` (needs that
  toolchain installed via rustup).
- `make coverage` — `cargo llvm-cov` to `target/coverage/tests.lcov` (skips doctests;
  CI runs `cargo test --doc` separately).
- `make format` — `cargo fmt` (edition 2024 style).

## Testing

- **Tests are deterministic and offline by default.** `tests/fixtures/ip-ranges.json` is
  a small, real subset of the AWS data (17 prefixes, IPv4 and IPv6, duplicates merged by
  service). CLI tests run the binary with `--offline` against it (`awsipranges()` helper
  in `tests/awsipranges.rs`) and assert exact output. Library tests use it through
  `core::test_utils::FIXTURE_JSON`.
- **Test HTTP behavior with `core::test_utils::serve`**, a scripted loopback server
  (responses per connection: `HttpResponse::ok`, `::status`, `::Stall`), not the real URL.
- Hostname tests don't depend on DNS: CLI tests use `localhost` (hosts file) and
  `*.invalid` names (never resolve, RFC 6761); matching logic uses a fake resolver in
  `cli/resolve.rs` unit tests.
- Only a few smoke tests hit the network: `client::tests::test_get_ranges_from_aws`,
  `command_live_download`, the doctests, and the `lib_demo` example.
- Don't mutate process environment variables in tests; use
  `ClientBuilder::from_env(lookup)`. Write temporary files with `tempfile`.
- If you change the fixture, update the exact-output assertions that depend on it.

## Architecture

- `src/lib.rs` — public API re-exports. Crate docs `include_str!` the files
  `examples/lib_demo.rs` and `docs/lib_configuration_table.md`, so both must exist at
  build time (including in the Docker build context; see `.dockerignore`).
- `src/core/` — the library (`mod core` is private; everything public is re-exported
  from `lib.rs`):
  - `client.rs` — `Client`/`ClientBuilder`/`get_ranges()`/`CacheMode`. Blocking
    reqwest, a local cache with a freshness window, and exponential-backoff retries
    within a `retry_timeout` deadline (each request's timeout is the remaining budget).
    `CacheMode::Auto` falls back to a stale cache; `Refresh`/`Offline` never fall back.
    Configured from `AWSIPRANGES_*` env vars in `::new()` (via `from_env(lookup)`) and
    ignores them in `::default()`. Log failures that are returned as `Err` at warn or
    debug level, never error, because the caller reports them.
  - `aws_ip_ranges.rs` — `AwsIpRanges`: a `BTreeMap<IpNetwork, AwsIpPrefix>` plus
    region, network border group, and service sets of interned `Rc<str>`.
    Supernet search uses a bounded `BTreeMap::range` scan (`supernet_prefixes`); the
    lower bound is clamped so broad searches like `0.0.0.0/0` can't invert the range.
  - `filter.rs` — `FilterBuilder` → `Filter`. AND across filter kinds, OR within one.
    Unknown region, service, or group values return `Err`.
  - `json.rs` / `datetime.rs` — zero-copy serde structs for the AWS JSON and its
    `%Y-%m-%d-%H-%M-%S` date format.
  - `errors.rs` — `#[non_exhaustive]` `Error` enum (thiserror). Wrap third-party errors
    as a boxed `source` rather than exposing their types in the public API.
- `src/main.rs` + `src/cli/` — clap-derive CLI and output formatters (comfy-table,
  JSON, CSV). The CLI normalizes case: regions and groups are lowercased except
  `GLOBAL`, and services are uppercased. Exit status follows grep: 0 = match,
  1 = no match, 2 = error. `main` prints errors with their causes and a hint, and
  treats a broken pipe as success. Output functions write to `&mut impl Write`; don't
  use `println!` (it panics when stdout is closed, e.g. `| head`).
  - `cli/target.rs` — `SearchTarget` (clap value parser): an IP/CIDR, else a validated
    hostname, else an "invalid input" error (exit 2). Hostnames whose last label is all
    digits are rejected so malformed IPv4 addresses (`1.2.3`) aren't looked up.
  - `cli/resolve.rs` — resolves hostnames to A + AAAA addresses with the system
    resolver (`ToSocketAddrs`); the resolver is a function parameter, so tests pass
    a fake. Failed lookups are reported as "did not resolve to an IPv4 (A) or IPv6
    (AAAA) address" (getaddrinfo can't tell NXDOMAIN from no records, so don't claim
    either; the resolver's text is logged at warn for `-v`), other results still print,
    and exit is 2.
    Hostname resolution is CLI-only; the library stays DNS-free.
  - `cli/search.rs` — `Search` (a target and its networks) and `matches()`, which maps
    each displayed AWS prefix back to the searches (and resolved addresses) it
    contains. Table/CSV add a Matches column and JSON a `matches` field only when
    searching; `-o cidr`/`netmask` stay one value per line for piping.

`Rc<str>` makes `AwsIpRanges` `!Send`/`!Sync`. Changing it to `Arc<str>` affects the
public API.

## Conventions

- Rust 2024 edition; MSRV in `Cargo.toml` `rust-version`. If a dependency bump raises
  the MSRV, update `rust-version` and the README "Build from source" note together.
- Match the existing style: banner comment blocks (`/*---...---*/`) separate sections,
  doc comments on every public item, and a doctest for each public API example.
- **Conventional Commits are required.** release-plz and git-cliff (`cliff.toml`)
  generate `CHANGELOG.md` and version bumps from commit messages. Don't edit
  `CHANGELOG.md` or the crate version by hand.
- Public API changes are semver-relevant (the crate is pre-1.0, so breaking changes
  bump the minor version). Mark breaking commits with `!` or a `BREAKING CHANGE:`
  footer.

## CI / release

- `.github/workflows/tests.yml` — Lint, MSRV, and Tests jobs. It runs on non-main
  pushes and PRs, and `release-crate.yml` calls it on main.
- `release-crate.yml` — on push to main, release-plz opens or updates the release PR
  and publishes to crates.io once that PR merges.
- `release.yml` — **generated by dist** (`cargo-dist-version` in `Cargo.toml`). Don't
  hand-edit it except for Dependabot action bumps (`allow-dirty = ["ci"]` permits
  those). To upgrade dist, bump the version in `Cargo.toml`, temporarily remove
  `allow-dirty`, and run `dist generate --mode=ci`.
  Afterwards, diff the regenerated workflow: dist uses `tap` in `Cargo.toml` verbatim as
  the tap repo, so it must be the full repo name (`cmlccie/homebrew-tap`).
- `.github/mergify.yml` auto-merges Dependabot PRs only after the Lint, MSRV, and
  Tests checks succeed. If you rename a CI job, update these check names.
