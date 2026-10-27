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
- `cargo +1.85 check --workspace --exclude itinera-conformance`
- `cargo doc --workspace --no-deps`, with `RUSTDOCFLAGS=-D warnings`
- `cargo deny check`

These are also required, and build `itinera` and `itinera-core` once for each combination of their features:

- `cargo hack clippy -p itinera -p itinera-core --feature-powerset --exclude-features default --all-targets -- -D warnings`
- `cargo hack test -p itinera --feature-powerset --exclude-features default,testing,unstable`
- `cargo hack test -p itinera-core --feature-powerset --exclude-features unstable`
- `cargo hack doc -p itinera -p itinera-core --feature-powerset --exclude-features default --no-deps`, with `RUSTDOCFLAGS=-D warnings`

They take several minutes, so before pushing they are run only when a change adds, moves or removes a feature gate, changes code under one, names an item under one, in code or in a documentation link, or changes a crate's features or dependencies. CI runs them on every pull request.

`cargo xtask check` runs the first list, in this order, and stops at the first that fails. `cargo xtask check --feature-combinations` runs the second list after it. The `cargo hack` checks need [cargo-hack](https://github.com/taiki-e/cargo-hack), and `cargo deny check` needs cargo-deny.

The conformance runner turns on `async`, `macros` and `testing` for every crate built with it, so the `cargo hack` checks build only `itinera` and `itinera-core`. The tests leave out `unstable`, which the tests with every feature on cover, and `itinera`'s tests always use `testing`, as an application's tests do.

## Conformance

`conformance.json` pins the conformance cases, lists the proposals implemented and the capabilities claimed, and maps each scenario excluded as impossible to express to the test that proves it. A proposal is listed only by the pull request that completes it.

In CI, the `run-conformance` action of [itinera-dev/actions](https://github.com/itinera-dev/actions) reads `conformance.json`, checks the excluded scenarios, runs the scenarios whose proposals are all listed, and writes the report. While no proposal is listed, it checks the manifest without starting the runner.

To run the cases yourself, check out [itinera-dev/conformance](https://github.com/itinera-dev/conformance) at the tag pinned in `conformance.json`, then run `cargo run -p itinera-conformance --release` from this repository with:

- `ITINERA_CONFORMANCE_CASES` set to that checkout's `cases` directory;
- `ITINERA_CONFORMANCE_TAGS` set to a Cucumber tag expression selecting the scenarios to run;
- `ITINERA_CONFORMANCE_REPORT` set to the file the Cucumber JSON report is written to.

## Commit messages and the changelog

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/), and each commit that changes what an application meets carries its changelog entries as trailers, which [git-cliff](https://git-cliff.org/) gathers into a release's section of `CHANGELOG.md`. For example:

```text
feat(step)!: give an attempt a type of its own

An attempt's number was a bare NonZeroU32, so nothing said that it
counted from 1 or named the first attempt.

Added: `step::Attempt`, an attempt's number, counted from 1, with `Attempt::FIRST` and `is_first()`.
Changed: An attempt's number is a `step::Attempt` instead of a `NonZeroU32` wherever it is public.
Refs #7
```

1. The subject is `type(scope): description`, or `type: description`. The types are `feat`, `fix`, `refactor`, `perf`, `docs`, `test`, `ci`, `build`, `chore` and `revert`, and the scope is a module or an area, such as `step`, `macros`, `release` or `book`. A `!` before the colon marks a change that breaks code written for the last release.
2. The body, after a blank line, says what changed and why.
3. The trailers close the message: one `Added:`, `Changed:`, `Fixed:` or `Removed:` line for each changelog entry, then `Refs #N` or `Closes #N`. An entry says what an application meets, by its public path, and for a breaking change what replaces what is gone. Changes to CI, the release workflow or the documentation have entries only when they change what is published or how.
4. A `feat`, `fix` or breaking commit with no entry, such as a fix to work that no release has shipped yet, says `Changelog: none`.

The required `commit messages` check runs the `commit-messages` action of [itinera-dev/actions](https://github.com/itinera-dev/actions) on every commit of a pull request other than merges. It refuses a subject of another form, a type outside the list, a body that does not follow a blank line, a `feat`, `fix` or breaking commit with neither entries nor `Changelog: none`, entries together with `Changelog: none`, and any entry that git-cliff would not read, would cut short or would read with more attached, such as an entry on several lines or a misspelt section. It does not check the wording. Before pushing, `PYTHONPATH=<actions> python3 <actions>/commit-messages/main.py origin/main..HEAD`, run in this repository with `<actions>` a checkout of itinera-dev/actions and Python 3.11 or later, checks the local commits the same way.

Pull requests merge with a merge commit, never squashed, because a squash folds every commit's trailers into one message. A commit's entries cannot change once it merges, so the release pull request reconciles entries that a later commit made untrue.

`cliff.toml` holds git-cliff's template. `git cliff --unreleased --tag v<version> --strip all` prints the section for `<version>` from the entries of the commits since the last release. The template is written for git-cliff 2.14.2, which installs with `cargo install git-cliff --version 2.14.2 --locked` or `pip install git-cliff==2.14.2`.

## Releasing

All three published crates share the workspace version and are published together. A maintainer requests a release by pushing, on `main`, an annotated tag `v` followed by that version, such as `v0.1.0-beta.1`, with the message `itinera <version>`, after the pull request that sets the version and adds its section to `CHANGELOG.md` has merged. The release workflow, `.github/workflows/release.yml`, then runs four jobs, each waiting for the one before:

1. **release gate**: the `release-gate` action refuses the tag unless it matches the version in `Cargo.toml`, the commit is on `main`, `CHANGELOG.md` has a section for the version, and `conformance.json` lists every tier 1 proposal of the pinned cases, except one whose every case needs a capability it does not claim;
2. **CI**: the whole of `.github/workflows/ci.yml` runs again on the tagged commit, since its run on `main` after a merge blocks nothing, and its conformance job keeps the report;
3. **crates.io**: in the `crates-io` environment, which only tags `v*` may use, the `publish-crates` action of [itinera-dev/actions](https://github.com/itinera-dev/actions) exchanges the workflow's identity for a short-lived crates.io token through trusted publishing, reads from crates.io which versions each crate has, and publishes with `cargo publish --locked` every crate of the workspace whose version is not there yet;
4. **GitHub release**: the `create-release` action creates the release with the notes of the version's changelog section, as a pre-release for a version such as `0.1.0-beta.1`, and attaches the conformance report.

Before pushing the tag, the maintainer runs `cargo publish --workspace --dry-run` at the commit to be tagged, since a tag whose publish job fails has no release. crates.io trusts only `release.yml` in this repository, running in the `crates-io` environment, so no token is stored anywhere. crates.io accepts new versions of these crates only through trusted publishing. If the job stops after publishing some of the crates, the maintainer re-runs the workflow's failed jobs: the publish job leaves the crates already on crates.io as they are, publishes the rest, and the GitHub release follows.

Cargo packages only a crate's own directory, so each published crate holds its own copy of `LICENSE-MIT` and `LICENSE-APACHE`. A change to either file at the root of the repository is made to every copy.

The itinera package leaves out its `tests/` directory, so packaging warns that it ignores each of its test targets. The warning is expected, and the tests still run in the repository.

## The book

[The Itinera book for Rust](https://itinera-dev.org/book/rust/) lives in [itinera-dev/book](https://github.com/itinera-dev/book), under `rust/`, and every example in it compiles and runs. `book.json` pins the book's commit that this repository is checked against. The required `book` check runs the `check-book` action of [itinera-dev/actions](https://github.com/itinera-dev/actions): it takes the book at that commit, builds its examples against this checkout's own crates instead of the revision the book names, and runs them with `cargo test`, so a change that breaks the book fails in its own pull request.

The book has two branches. Its `main` follows the latest release of itinera from crates.io, and it is what the published book shows. Its `next` branch follows this repository's `main` by git revision, and `book.json` pins a commit on `next`.

A change that breaks the book comes with a paired pull request in the book, against `next`, that fixes it. The change's own pull request moves `book.json` to the commit of the book's fix, and the book's pull request moves `next`'s revision of itinera to the change once it merges. If the book's pull request is squashed when it merges, a follow-up moves `book.json` to the book's merge commit, so that the pin stays on `next`.

A pull request that only sets a new version needs no change in the book, since `next`'s dependency on itinera names a revision rather than a version, but it moves `book.json` to `next`'s head, so that the book `main` will take is checked against the release. Once the release is on crates.io, the book:

1. merges into its `main` the last commit of `next` whose revision of itinera is the released commit or an earlier one, raising `main`'s requirement to exactly the new version, `=<version>`, keeping `main`'s dependencies on crates.io and their comments, and regenerating its lock file from them;
2. raises, on `main`, the prose that names the release to the new version, such as the installing chapter and the appendix on what the version supports;
3. is tagged by the maintainer on the commit that finishes this, with an annotated tag `rust-<version>` and the message `The Rust book for itinera <version>`, which the published book keeps as that version's edition;
4. merges `main` back into `next`, keeping `next`'s dependency lines and lock file.
