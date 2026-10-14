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

All three published crates share the workspace version and are published together. A maintainer requests a release by pushing a tag `v` followed by that version, such as `v0.1.0-beta.1`, on `main`, after the pull request that sets the version and adds its section to `CHANGELOG.md` has merged. The release workflow, `.github/workflows/release.yml`, then runs three jobs, each waiting for the one before:

1. **release gate**: the `release-gate` action refuses the tag unless it matches the version in `Cargo.toml`, the commit is on `main`, `CHANGELOG.md` has a section for the version, and `conformance.json` lists every tier 1 proposal of the pinned cases;
2. **conformance**: `run-conformance` runs the cases and keeps the report;
3. **GitHub release**: the `create-release` action creates the release with the notes of the version's changelog section, as a pre-release for a version such as `0.1.0-beta.1`, and attaches the conformance report.

For now, the crates are published by hand. Before pushing the tag, the maintainer runs `cargo publish --workspace --dry-run` at the commit to be tagged, since the workflow creates the GitHub release before anything is published. Once the workflow has finished and the release exists, the maintainer checks out the tagged commit and runs `cargo publish --workspace`, logged in with `cargo login` on their own machine. crates.io configures trusted publishing only for crates that already exist, so once the crates are published, trusted publishing is configured for them and a publish job joins the workflow.

## The book

[The Itinera book for Rust](https://itinera-dev.org/book/rust/) lives in [itinera-dev/book](https://github.com/itinera-dev/book), under `rust/`, and every example in it compiles and runs. `book.json` pins the book's commit that this repository is checked against. The required `book` check runs the `check-book` action of [itinera-dev/actions](https://github.com/itinera-dev/actions): it takes the book at that commit, builds its examples against this checkout's own crates instead of the version the book names, and runs them with `cargo test`, so a change that breaks the book fails in its own pull request.

A change that breaks the book comes with a paired pull request in the book that fixes it. The change's own pull request moves `book.json` to the commit of the book's fix, and the book's pull request moves the book's pin of itinera to the change once it merges. If the book's pull request is squashed when it merges, a follow-up moves `book.json` to the book's merge commit, so that the pin stays on the book's `main`.
