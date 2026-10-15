---
name: release
description: How Claude prepares a release of itinera, itinera-core and itinera-macros, and what follows it in the book and the site. Use when the maintainer asks for a release, when writing a version's changelog section, and once a release reaches crates.io.
---

# Releasing

The "Releasing" section of [docs/contributing.md](../../../docs/contributing.md) says how a release runs: the tag, the release gate, the release workflow and its four jobs, trusted publishing, and what to do when publishing stops halfway. Its "The book" section says how the book's two branches follow this repository and a release. This skill adds only the part Claude does.

Claude never pushes a tag and never publishes to crates.io. The maintainer pushes the tag, and the release workflow publishes.

## The release pull request

One pull request, titled "Release <version>" with one commit of the same name, sets the version and writes its changelog section. It names its issue as AGENTS.md says, normally `Refs #7`.

1. Branch from the latest `main`.
2. After `git fetch --tags`, list what changed since the last release with `git log v<last version>..origin/main`, and read each commit's message.
3. Raise the version:
   - in `Cargo.toml`, the workspace version and the `version` of the `itinera-core` and `itinera-macros` path dependencies;
   - in `Cargo.lock`, with `cargo update --workspace`;
   - in the status paragraph of `README.md`.
4. Check that the commit to be tagged will meet every condition of the release gate in docs/contributing.md "Releasing". In particular, `conformance.json` must list every tier 1 proposal of the cases it pins. If it does not, stop and tell the maintainer, since the tag would be refused.
5. Write the version's section at the top of `CHANGELOG.md`, in the form of the sections below it:
   - a heading `## [<version>]`, and its link at the bottom of the file, to `releases/tag/v<version>`;
   - one paragraph naming which release it is, whether it breaks code written for the last one, the specification version, the tier, the capabilities, the conformance cases of `conformance.json` and the minimum supported Rust version, and saying "What a journey does is unchanged" when that is so;
   - `### Added`, `### Changed`, `### Fixed` or `### Removed`, each only when it has entries.
6. Each entry says what an application meets, by its public path, such as `journey::Read::present`:
   - every breaking change, with what replaces what is gone;
   - nothing that was already true in the last release, which `git show v<last version>:<file>` settles;
   - nothing about how the documentation spells something the language already allowed, such as a lifetime the language lets code leave out;
   - CI and release changes only when they change what is published or how it is published, such as docs.rs features or the release workflow.
7. Move `book.json` as "The book" in docs/contributing.md says. `git ls-remote https://github.com/itinera-dev/book refs/heads/next` gives `next`'s head.
8. Run the checks of AGENTS.md "Running the checks", and have the change reviewed as the `review-change` skill says. Ask the reviewer to check every changelog entry against the code at both versions.
9. Open the pull request, in a cloud session with the REST form the `stack` skill gives, and report it as AGENTS.md "Working with the maintainer" says.

## After the merge

1. Tell the maintainer the commit to tag, `main`'s merge commit, and that `cargo publish --workspace --dry-run` at that commit comes before the tag, as docs/contributing.md says.
2. Once the release workflow has finished, check that:
   - `itinera`, `itinera-core` and `itinera-macros` are on crates.io at the version, and docs.rs built them;
   - the GitHub release exists, marked as a pre-release when the version has a pre-release part such as `-beta.3`, with the changelog section and the conformance report.

   If publishing stopped halfway, tell the maintainer, pointing at the recovery in docs/contributing.md "Releasing".
3. The book follows the release as "The book" in docs/contributing.md says. Claude does the book's part from a thread working on the book, or asks the maintainer whether one should; the maintainer tags the book.
4. The site, itinera-dev/itinera-dev.github.io, takes each language's version from its `src/data/languages.json`. Update it from a thread working on the site, or tell the maintainer it follows the release.
