---
name: release
description: How Claude prepares a release of itinera, itinera-core and itinera-macros, and what follows it in the book. Use when the maintainer asks for a release, when writing a version's changelog section, and once a release reaches crates.io.
---

# Releasing

The "Releasing" section of [docs/contributing.md](../../../docs/contributing.md) says how a release runs: the tag, the release workflow and its four jobs, trusted publishing, and what to do when publishing stops halfway. Its "The book" section says how the book's two branches follow this repository. This skill adds only the part Claude does.

Claude never pushes a tag and never publishes to crates.io. The maintainer pushes the tag, and the release workflow publishes. Cloud sessions cannot create tags at all.

## The release pull request

One pull request sets the version and writes its changelog section. It says `Refs #7`, or `Closes #N` for an issue of the release's own.

1. Branch from the latest `main`.
2. List what changed since the last release with `git log v<last version>..origin/main`, and read each commit's message.
3. Raise the version:
   - in `Cargo.toml`, the workspace version and the `version` of the `itinera-core` and `itinera-macros` path dependencies;
   - in `Cargo.lock`, with `cargo update --workspace`;
   - in the status paragraph of `README.md`.
4. Write the version's section at the top of `CHANGELOG.md`, in the form of the sections below it:
   - a heading `## [<version>]`, and its link at the bottom of the file, to `releases/tag/v<version>`;
   - one paragraph naming which release it is, whether it breaks code written for the last one, the specification version, the tier, the capabilities, the conformance cases of `conformance.json` and the minimum supported Rust version, and saying "What a journey does is unchanged" when that is so;
   - `### Added`, `### Changed`, `### Fixed` or `### Removed`, each only when it has entries.
5. Each entry says what an application meets, by its public path, such as `journey::Read::present`:
   - every breaking change, with what replaces what is gone;
   - nothing that was already true in the last release, which `git show v<last version>:<file>` settles;
   - nothing about how the documentation spells something the language already allowed, such as a lifetime it lets you leave out;
   - CI and release changes only when they change what is published, such as docs.rs features.
6. Check `book.json`. It pins a commit on the book's `next` branch. When `next` has moved past the pin, move the pin to `next`'s head, which `git ls-remote https://github.com/itinera-dev/book refs/heads/next` gives, since this pull request is the last check of the book before the release.
7. Run the checks of AGENTS.md "Running the checks", and have the change reviewed as the `review-change` skill says. Ask the reviewer to check every changelog entry against the code at both versions.
8. Open the pull request, and tell the maintainer it is ready only once the review and CI have passed.

## After the merge

1. Tell the maintainer the commit to tag, `main`'s merge commit, and that `cargo publish --workspace --dry-run` at that commit comes before the tag, as docs/contributing.md says.
2. Once the release workflow has finished, check that the GitHub release exists as a pre-release when the version has one, with the changelog section and the conformance report.
3. The book follows the release, in itinera-dev/book: its `main` merges the last commit of `next` whose revision of itinera is the released commit or an earlier one, keeping `main`'s crates.io requirement, raised to `=<version>`. The maintainer then tags `rust-<version>` on that book commit. Claude does this from a thread working on the book, or asks the maintainer whether one should.
