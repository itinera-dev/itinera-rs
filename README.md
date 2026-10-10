# itinera-rs

The Rust implementation of [Itinera](https://github.com/itinera-dev/spec), a workflow framework that keeps business rules separate from flow control.

## Status

Not released yet. The first release, `0.1.0-rc.1`, will implement tier 1 of specification 0.1.0, with the capabilities `sync` and `async`.

## Using it

Applications depend on one crate, `itinera`. Its features are:

| Feature | Default | Adds |
|---|---|---|
| `macros` | on | The macros for declaring steps, policies and workflows, which are syntax over the builder |
| `async` | off | The asynchronous executor, and asynchronous steps, hooks, reporters and dispatchers |
| `unstable` | off | API the specification has not settled yet, which may change or go: for now, an input adapter's read access to the data bag |

`itinera` brings in `itinera-core`, which holds the implementation, and, with `macros`, `itinera-macros`. Neither needs to be added directly. The minimum supported Rust version is 1.85.

## Rules made impossible to express

The specification lets an implementation make some rules impossible to express rather than check them while a workflow runs. In Rust, these mistakes are compile errors:

- a hook returning a lifecycle it may not return;
- a policy needing a role the workflow does not provide;
- an asynchronous part run by the synchronous executor;
- something that is not a value entering the data bag, event data or a reason's details;
- a contributor or reporter used after its attempt or hook has ended.

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
