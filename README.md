# itinera-rs

The Rust implementation of [Itinera](https://github.com/itinera-dev/spec), a workflow framework that keeps business rules separate from flow control.

## Status

Tier 1 of specification 0.1.0 is being implemented, following the [tier 1 plan](docs/tier-1-plan.md), tracked in [#7](https://github.com/itinera-dev/itinera-rs/issues/7). Nothing is released yet, and no proposal is listed in [conformance.json](conformance.json), so no conformance case runs yet.

When the plan is complete, the first release, `0.1.0-rc.1`, will claim:

- specification 0.1.0, tier 1;
- the capabilities `sync` and `async`;
- the conformance cases pinned in `conformance.json`, currently `v0.1.0-rc.2`.

## Crates

A Cargo workspace under `crates/`:

| Crate | Published | Holds |
|---|---|---|
| `itinera` | yes | What applications depend on: everything in `itinera-core`, and the macros behind the `macros` feature |
| `itinera-core` | yes | Values and the data bag, events, reporters and dispatchers, steps, policies, workflow descriptors, workflow instances and the local executors |
| `itinera-macros` | yes | The procedural macros, which are syntax over the builder |
| `itinera-conformance` | no | The conformance runner |

The features of `itinera` are `macros` (on by default), `async` (the asynchronous executor and everything asynchronous) and `unstable` (the API of proposals not yet listed in `conformance.json`).

## Rules made impossible to express

The specification lets a language make some rules impossible to express rather than check them while running. In Rust these are made impossible by types, each proven by a compile-fail test:

- `invalid-lifecycle`: a hook returning a lifecycle it may not return;
- `role-not-provided`: a policy needing a role the workflow does not provide;
- `mode-not-accepted`: an asynchronous part run by the synchronous executor;
- `non-value`: something that is not a value entering the data bag, event data or a reason's details;
- `late-handle`: a contributor or reporter used after its attempt or hook.

`conformance.json` lists each scenario excluded this way, with the test that proves it, once that test exists.

## Building and testing

The toolchain is pinned in `rust-toolchain.toml`; the minimum supported Rust version is 1.85. The checks CI runs are:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo check --workspace --no-default-features`
- `cargo +1.85 check --workspace`
- `cargo doc --workspace --no-deps`, with `RUSTDOCFLAGS=-D warnings`
- `cargo deny check`

## Running the conformance cases

The runner is started with `cargo run -p itinera-conformance --release`. It reads three environment variables:

- `ITINERA_CONFORMANCE_CASES`: the directory holding the cases of [itinera-dev/conformance](https://github.com/itinera-dev/conformance), at the tag pinned in `conformance.json`;
- `ITINERA_CONFORMANCE_TAGS`: the Cucumber tag expression selecting the scenarios to run;
- `ITINERA_CONFORMANCE_REPORT`: the file the Cucumber JSON report is written to.

In CI, the `run-conformance` action of [itinera-dev/actions](https://github.com/itinera-dev/actions) sets them from `conformance.json`, checks the excluded scenarios and writes the report. While no proposal is listed it does not start the runner.

## How work arrives here

What Itinera does is defined in [itinera-dev/spec](https://github.com/itinera-dev/spec): the proposals (the PRDs) and the behaviour specification. How Rust does it is defined here, in **tech specs**.

1. Each accepted proposal gets an **implementation issue** here, labelled `implements-proposal`. An issue opened before its proposal is accepted also carries `waiting for spec`, and no pull request is opened for it until that label is removed.
2. The Rust **tech spec** is agreed in that issue: API shape, crates and modules touched, types, macros, error handling, tests and the definition of done.
3. The implementing pull request adds the tech spec as `docs/specs/NNNN-short-name.md`, where `NNNN` is the proposal's number, together with the code.
4. The issue closes when the proposal's [conformance](https://github.com/itinera-dev/conformance) cases pass.

Bugs and Rust API questions can be opened here directly; changes to behaviour are proposals in the spec repository. The full rules are in [PROCESS.md](https://github.com/itinera-dev/spec/blob/main/PROCESS.md).

Built with AI under the terms of [A manifesto for software engineering with AI](https://marlon-sousa.com/blog/manifesto/); see [how Itinera is built](https://github.com/itinera-dev/.github/blob/main/CONTRIBUTING.md#how-itinera-is-built).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
