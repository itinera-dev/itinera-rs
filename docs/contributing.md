# Contributing to itinera-rs

This guide covers what is specific to the Rust implementation. The rules every itinera-dev repository follows are in the [organisation's contributing guide](https://github.com/itinera-dev/.github/blob/main/CONTRIBUTING.md), and the full process is in [PROCESS.md](https://github.com/itinera-dev/spec/blob/main/PROCESS.md).

## How work arrives here

What Itinera does is defined in [itinera-dev/spec](https://github.com/itinera-dev/spec): the proposals (the PRDs) and the behaviour specification. How Rust does it is defined here, in **tech specs**.

1. Each accepted proposal gets an **implementation issue** here, labelled `implements-proposal`. An issue opened before its proposal is accepted also carries `waiting for spec`, and no pull request is opened for it until that label is removed.
2. The Rust **tech spec** is agreed in that issue: API shape, crates and modules touched, types, macros, error handling, tests and the definition of done.
3. The implementing pull request adds the tech spec as `docs/specs/NNNN-short-name.md`, where `NNNN` is the proposal's number, together with the code.
4. The issue closes when the proposal's [conformance](https://github.com/itinera-dev/conformance) cases pass.

Bugs and Rust API questions can be opened here directly; changes to behaviour are proposals in the spec repository.

## The plan being implemented

Tier 1 is implemented following the [tier 1 plan](tier-1-plan.md), in stages tracked in [#7](https://github.com/itinera-dev/itinera-rs/issues/7). The plan holds the architecture, the tests, the order of work and the tooling; each proposal's tech spec holds what concerns that proposal alone. [architecture.md](architecture.md) draws what has been built, tier by tier, and is kept true by every pull request.

## The workspace

| Crate | Published | Holds |
|---|---|---|
| `itinera` | yes | What applications depend on: everything in `itinera-core`, and the macros behind the `macros` feature |
| `itinera-core` | yes | Values and the data bag, events, reporters and dispatchers, steps, policies, workflow descriptors, workflow instances and the local executors |
| `itinera-macros` | yes | The procedural macros, which are syntax over the builder |
| `itinera-conformance` | no | The conformance runner |
| `xtask` | no | Development tasks, such as `cargo xtask check` |

The `unstable` feature holds API that the specification has not accepted yet, and, before a proposal is listed in `conformance.json`, that proposal's API. It holds nothing for now.

## Building and testing

The toolchain is pinned in `rust-toolchain.toml`, and the minimum supported Rust version of the published crates is 1.85. The conformance runner, which is not published, needs 1.88. Every one of these is a required check in CI:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo check --workspace --no-default-features`
- `cargo +1.85 check --workspace --exclude itinera-conformance`
- `cargo doc --workspace --no-deps`, with `RUSTDOCFLAGS=-D warnings`
- `cargo deny check`

`cargo xtask check` runs them all, in this order, and stops at the first that fails.

## Conformance in CI

`conformance.json` pins the conformance cases, lists the proposals implemented and the capabilities claimed, and maps each scenario excluded as impossible to express to the test that proves it. A proposal is listed only by the pull request that completes it.

In CI, the `run-conformance` action of [itinera-dev/actions](https://github.com/itinera-dev/actions) reads `conformance.json`, checks the excluded scenarios, runs the scenarios whose proposals are all listed, and writes the report. While no proposal is listed, it checks the manifest without starting the runner.

## Releasing

All three published crates share the workspace version and are published together, only by the release workflow, `.github/workflows/release.yml`. A maintainer requests a release by pushing a tag `v` followed by that version, such as `v0.1.0-rc.1`, on `main`, after the pull request that sets the version and adds its section to `CHANGELOG.md` has merged. The workflow then runs four jobs, each waiting for the one before:

1. **release gate**: the `release-gate` action refuses the tag unless it matches the version in `Cargo.toml`, the commit is on `main`, `CHANGELOG.md` has a section for the version, and `conformance.json` lists every tier 1 proposal of the pinned cases;
2. **conformance**: `run-conformance` runs the cases and keeps the report;
3. **publish to crates.io**: `cargo publish --workspace`, in the `crates-io` environment, which waits for a maintainer's approval;
4. **GitHub release**: the `create-release` action creates the release with the notes of the version's changelog section, as a pre-release for a version such as `0.1.0-rc.1`, and attaches the conformance report.

The first release of each crate is published with a crates.io token that a maintainer creates for that release only and stores as the `CARGO_REGISTRY_TOKEN` secret of the `crates-io` environment, because crates.io configures trusted publishing only for crates that already exist. Once that release is published, trusted publishing is configured for the three crates, the token is deleted, and the publish job authenticates through trusted publishing from then on.
