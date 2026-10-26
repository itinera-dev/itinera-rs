# itinera-rs

The Rust implementation of [Itinera](https://github.com/itinera-dev/spec), a workflow framework that keeps business rules separate from flow control.

## Learning it

[The Itinera book for Rust](https://itinera-dev.org/book/rust/) teaches Itinera from a first workflow onwards, and every example in it compiles and runs. [The Itinera site](https://itinera-dev.org/) explains why Itinera exists. The crate's documentation on [docs.rs](https://docs.rs/itinera) is the reference for each item.

## Status

`0.1.0-beta.3` implements tier 1 of specification 0.1.0, with the capabilities `sync`, `async` and `testing`, and passes the conformance cases `v0.1.0-rc.15`. It is a beta, not yet meant for production: until `0.1.0`, features may be added and its API may still change. [CHANGELOG.md](https://github.com/itinera-dev/itinera-rs/blob/main/CHANGELOG.md) says what each release implements.

## Using it

Applications depend on one crate, `itinera`. Its features are:

| Feature | Default | Adds |
|---|---|---|
| `macros` | on | The macros for declaring steps, rescue steps, policies, workflows and roles, which are syntax over the builder |
| `async` | off | The asynchronous executor, and asynchronous steps, rescue steps, hooks, service providers, reporters and dispatchers, with the `async_trait` attribute for services and roles whose methods are asynchronous |
| `testing` | off | Test support: `RecordingReporter`, which keeps every event of a journey, and the latest value of each entry of its global metadata, for a test to read, fake service providers, scripted runs, and recorders that lend a step, a hook or an input adapter its handles outside a journey; enable it under `[dev-dependencies]` |
| `unstable` | off | API the specification has not settled yet, which may change or go; empty for now |

`itinera` brings in `itinera-core`, which holds the implementation, and, with `macros` or `async`, `itinera-macros`. Neither needs to be added directly. The minimum supported Rust version is 1.85.

The crate's documentation opens with a first complete workflow. [crates/itinera/examples](https://github.com/itinera-dev/itinera-rs/tree/main/crates/itinera/examples) holds three runnable ones: `orders`, a synchronous workflow declared with the macros; `orders_async`, the same on the asynchronous executor, which needs the `async` feature; and `orders_by_hand`, the same declared with the builder alone. Each charges through a payment service. Run one with `cargo run -p itinera --example orders`.

## Rules made impossible to express

The specification lets an implementation make some rules impossible to express rather than check them while a workflow runs. In Rust, these mistakes are compile errors:

- a hook returning a lifecycle it may not return;
- a policy needing a role the workflow does not provide;
- an asynchronous part run by the synchronous executor, or an asynchronous service provider in a synchronous workflow;
- a rescue step that skips;
- a service provider of journey scope requesting the step being built or the attempt, or a service provider returning nothing instead of an instance or an error;
- something that is not a value entering the data bag, event data, a reason's details or global metadata;
- a contributor or reporter used after its attempt or hook has ended, or an input adapter's access to the data bag used after its call;
- an input adapter writing to the data bag through its access;
- a test's script choosing a negative attempt or a fraction of one.

## How it works

[docs/architecture.md](https://github.com/itinera-dev/itinera-rs/blob/main/docs/architecture.md) draws how itinera-rs is put together and how it runs a journey: the crates and the macros, the executor loop, how each step is built and run, how its inputs and services reach it, what follows each outcome, and where policies, hooks, events and reporters come in.

## Conformance

Each release states the specification version, tier, capabilities and conformance cases it implements, and carries the report of those cases as a release asset.

To run the cases yourself, check out [itinera-dev/conformance](https://github.com/itinera-dev/conformance) at the tag pinned in [conformance.json](https://github.com/itinera-dev/itinera-rs/blob/main/conformance.json), then run `cargo run -p itinera-conformance --release` from this repository with:

- `ITINERA_CONFORMANCE_CASES` set to that checkout's `cases` directory;
- `ITINERA_CONFORMANCE_TAGS` set to a Cucumber tag expression selecting the scenarios to run;
- `ITINERA_CONFORMANCE_REPORT` set to the file the Cucumber JSON report is written to.

## Contributing

See the [contributor guide](https://github.com/itinera-dev/itinera-rs/blob/main/docs/contributing.md) for how work arrives here, how to build and test, and the plan being implemented, and the [organisation's contributing guide](https://github.com/itinera-dev/.github/blob/main/CONTRIBUTING.md) for the rules every itinera-dev repository follows.

Built with AI under the terms of [A manifesto for software engineering with AI](https://marlon-sousa.com/blog/manifesto/); see [how Itinera is built](https://github.com/itinera-dev/.github/blob/main/CONTRIBUTING.md#how-itinera-is-built).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/itinera-dev/itinera-rs/blob/main/LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](https://github.com/itinera-dev/itinera-rs/blob/main/LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
