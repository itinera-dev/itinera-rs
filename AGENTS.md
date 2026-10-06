# Agents' manual

How to work in itinera-rs, the Rust implementation of [Itinera](https://github.com/itinera-dev/spec). This file is self-contained; read it before changing anything.

## Where behaviour comes from

- Behaviour comes only from the specification in [itinera-dev/spec](https://github.com/itinera-dev/spec), and the conformance cases in [itinera-dev/conformance](https://github.com/itinera-dev/conformance) check it.
- If the specification has a gap or a contradiction, stop and tell the maintainer. Never decide it in Rust.
- How Rust implements tier 1 is decided in [docs/tier-1-plan.md](docs/tier-1-plan.md). What concerns a single proposal goes in its tech spec, `docs/specs/NNNN-short-name.md`. Where the plan and the specification disagree, the specification is right and the plan is corrected.

## Stages

The plan is implemented in stages, tracked in [#7](https://github.com/itinera-dev/itinera-rs/issues/7):

0. Bootstrap: the workspace, the toolchain, lints, CI, `deny.toml`, the README, this manual and `conformance.json`.
1. Events: `Event`, reporters, dispatchers and their factory, `DefaultDispatcher`.
2. Runner skeleton: cucumber-rs, the environment variables, the recording dispatcher factory, the scenario model, the Cucumber JSON report.
3. Minimal executor: the engine and both executors, a workflow descriptor with one synchronous step, workflow instances, journey IDs, reporters, the result and refusals.
4. Declarations and admission: step and policy descriptors, input adapter declarations, the listing, violations.
5. Steps: needs and tokens, building per attempt, outcomes, contributions, the data bag, values, handles and interruption.
6. The scan and its decisions: statuses, retries, abnormal termination, aborts and the result.
7. Policies, hooks and roles.
8. Macros.
9. Release of `0.1.0-rc.1`.

A proposal is added to `proposals` in `conformance.json` only by the pull request that completes it, which says `Closes #N` for its implementation issue. Proofs that a rule is impossible to express land with their feature, together with their entries under `impossible`.

## Code

- Formatting is plain `rustfmt`, without configuration.
- Names use the specification's vocabulary exactly.
- Library code never panics: Clippy denies `unwrap`, `expect`, `panic!`, indexing, `todo!` and `unimplemented!` outside tests.
- Everything is `pub(crate)` unless it must be public. Public types that may grow are `#[non_exhaustive]`.
- Every crate has `#![forbid(unsafe_code)]`.
- Every public item has documentation in British spelling, describing its behaviour in its own words, with an example that compiles and runs.
- Documentation is written where an item is defined; rustdoc carries it to every re-export. Examples use the paths applications write, through `itinera` (for example `use itinera::Value;`), never `itinera_core` or `itinera_macros`. For that, `itinera-core` has `itinera` as a dev-dependency. Cargo accepts the cycle because a dev-dependency only builds tests and examples, and `cargo package` drops it; `serde_core` does the same with `serde`. Unit tests inside `itinera-core` keep `crate::` paths, because a test build has its own copy of the crate's types, distinct from those `itinera` re-exports.
- Test names state the rule as a sentence, for example `a_failed_attempts_contributions_are_never_committed`.

## Comments

- As few as possible. A comment explains a non-obvious reason, only when the code cannot.
- Code and comments never refer to specification sections, issues, pull requests or other documents. Git keeps that history, and the tech specs map rules to code.
- No `TODO` comments.

## Running the checks

Each of these is a required CI check, and every one must pass before pushing:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo check --workspace --no-default-features`
- `cargo +1.85 check --workspace`
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`
- `cargo deny check`

## Running the conformance runner

Run `cargo run -p itinera-conformance --release` with:

- `ITINERA_CONFORMANCE_CASES` set to the `cases` directory of a checkout of itinera-dev/conformance at the tag pinned in `conformance.json`;
- `ITINERA_CONFORMANCE_TAGS` set to a Cucumber tag expression, for example `@proposal-0002 and not @non-value`;
- `ITINERA_CONFORMANCE_REPORT` set to the file the Cucumber JSON report is written to.

In CI the `run-conformance` action sets these from `conformance.json`.

## Pull requests and stacks

- Each stage is one stack of pull requests, made with `gh stack`, one layer per coherent piece. Every layer passes all of `main`'s checks.
- Every pull request names an open issue in this repository: `Refs #N` when it contributes, `Closes #N` when it finishes. Work that belongs to no proposal refers to #7, or to an issue of its own.
- Public API of a proposal not yet listed in `conformance.json` stays behind the `unstable` feature.
- Commit messages and pull request descriptions say what changed and why, in plain prose.

## Writing

- Everything is written for a screen reader: headings, lists, prose and simple tables.
- Never ASCII-art diagrams, arrows drawn with characters or box-drawing trees, in replies, documents or code. Diagrams are Mermaid, with `accTitle` and `accDescr`, and text saying everything they show.

## Working with the maintainer

- Start every reply with a heading.
- When a design decision is open, present one decision at a time, with a recommendation, and wait for the answer.
