# itinera-rs

The Rust implementation of [Itinera](https://github.com/itinera-dev/spec), a workflow framework that keeps business rules separate from flow control.

## Status

Not started. Itinera is being specified first; this repository will implement tier 1 once the core model proposal ([spec#2](https://github.com/itinera-dev/spec/issues/2)) is accepted.

## What it will contain

A Cargo workspace:

- `itinera`: the crate applications depend on, re-exporting everything below;
- `itinera-core`: outcomes, lifecycles, step statuses, the executor's scan, the data bag and the event model;
- `itinera-macros`: the step, workflow and policy macros;
- `itinera-executor-local`: in-memory executors, synchronous and asynchronous;
- later, a durable executor and store implementations.

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
