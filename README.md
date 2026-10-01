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

Behaviour is defined in [itinera-dev/spec](https://github.com/itinera-dev/spec). Each accepted proposal gets an issue here labelled `implements-proposal`, which closes when the proposal's [conformance](https://github.com/itinera-dev/conformance) cases pass. Bugs and Rust API questions can be opened here directly; changes to behaviour are proposals in the spec repository.

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
