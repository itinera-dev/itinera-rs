---
name: release
description: How Claude prepares a release of itinera, itinera-core and itinera-macros, and carries it into the book and the site. Use when the maintainer asks for a release, when writing a version's changelog section, and as soon as a release reaches crates.io.
---

# Releasing

The "Releasing" section of [docs/contributing.md](../../../docs/contributing.md) says how a release runs: the tag, the release gate, the release workflow and its four jobs, trusted publishing, and what to do when publishing stops halfway. Its "The book" section says how the book's two branches follow this repository and a release. This skill adds only the part Claude does.

Claude never pushes a tag and never publishes to crates.io. The maintainer pushes the tag, and the release workflow publishes.

## The release pull request

One pull request, titled "Release <version>" with a release commit `chore(release): release <version>`, sets the version and writes its changelog section. It names its issue as AGENTS.md says, normally `Refs #7`.

1. Branch from the latest `main`.
2. After `git fetch --tags`, list what changed since the last release with `git log v<last version>..origin/main`, and read each commit's message, its changelog trailers included.
3. Raise the version:
   - in `Cargo.toml`, the workspace version and the `version` of the `itinera-core` and `itinera-macros` path dependencies;
   - in `Cargo.lock`, with `cargo update --workspace`;
   - in the status paragraph of `README.md`, with the cases it passes.
4. Check that `conformance.json` lists every tier 1 proposal of the cases it pins, except one whose every case needs a capability it does not claim, which the release gate in docs/contributing.md "Releasing" requires. If it does not, stop and tell the maintainer, since the tag would be refused.
5. Write the version's section at the top of `CHANGELOG.md`, above the last release's, in the form of the sections below it:
   - `git cliff --unreleased --tag v<version> --strip all` prints the heading `## [<version>]` and the entries that commits since the last release carry as trailers, under `### Added`, `### Changed`, `### Fixed` and `### Removed`, each only when it has entries; `docs/contributing.md` "Commit messages and the changelog" says how;
   - after the heading, one paragraph naming which release it is, whether it breaks code written for the last one, the specification version, the tier, the capabilities, the conformance cases of `conformance.json` and the minimum supported Rust version, and saying "What a journey does is unchanged" when that is so;
   - the heading's link at the bottom of the file, to `releases/tag/v<version>`.
6. Reconcile the entries, since a commit's entries cannot change once it merges. Each entry follows AGENTS.md, and the section as a whole says what an application meets, compared with the last release:
   - entries that a later commit made untrue, such as an addition renamed before the release, become one entry saying what the release holds;
   - entries that say the same thing merge;
   - nothing that was already true in the last release, which `git show v<last version>:<file>` settles;
   - nothing about how the documentation spells something the language already allowed, such as a lifetime the language lets code leave out;
   - a commit since the last release that changes what an application meets but carries no entry, such as one merged before commits carried them, gets its entry here; changes to CI, the release workflow or the documentation get one only when they change what is published or how, as docs/contributing.md "Commit messages and the changelog" says.
7. Move `book.json` as "The book" in docs/contributing.md says. `git ls-remote https://github.com/itinera-dev/book refs/heads/next` gives `next`'s head.
8. Check the change against every condition of the release gate. Run the checks of AGENTS.md "Running the checks", and have the change reviewed as the `review-change` skill says. Ask the reviewer to check every changelog entry against the code at both versions, and the section against the entries git-cliff printed.
9. Open the pull request, in a cloud session with the REST form the `stack` skill gives, and report it as AGENTS.md "Working with the maintainer" says.

## After the merge

1. Give the maintainer the commands that make the tag docs/contributing.md "Releasing" describes, at `main`'s merge commit:
   1. `git fetch origin`
   2. `git switch --detach <commit>`
   3. `cargo publish --workspace --dry-run`
   4. `git tag -a v<version> <commit> -m "itinera <version>"`
   5. `git push origin v<version>`

   Once it is pushed, check with `git ls-remote origin 'refs/tags/v<version>*'` that it shows a tag object, and a `^{}` line naming the commit.
2. Once the release workflow has finished, check that:
   - `itinera`, `itinera-core` and `itinera-macros` are on crates.io at the version, and docs.rs built them;
   - the GitHub release exists, marked as a pre-release when the version has a pre-release part such as `-beta.3`, with the changelog section and the conformance report.

   If publishing stopped halfway, tell the maintainer, pointing at the recovery in docs/contributing.md "Releasing".

## Once the crates are published

As soon as all three crates are on crates.io at the version, Claude carries the release into the book and the site without waiting to be asked; a publish that stopped halfway is recovered first. The maintainer's standing preference is that both changes are pushed straight to each repository's `main`, with no pull request, once the checks its own manual asks for pass and a separate reviewer has passed them. When this session cannot reach a repository, Claude asks the maintainer for a thread working on it, and passes that preference on.

1. **The book**, itinera-dev/book, follows the release as the numbered list at the end of "The book" in docs/contributing.md says. Once the commit that raises the prose naming the release is on the book's `main`, give the maintainer the commands that tag it:
   1. `git fetch origin`
   2. `git tag -a rust-<version> <commit> -m "The Rust book for itinera <version>"`
   3. `git push origin rust-<version>`

   The tag freezes its commit as the version's edition, so it goes only on the commit whose prose names the new version.
2. **The site**, itinera-dev/itinera-dev.github.io, follows each language's release, as its own manual says. For Rust:
   - in `src/data/languages.json`, the entry's `state` becomes `released`, and `version`, `install`, `tier` and `executors` match the release and its `conformance.json`;
   - in the `facts` of `site.config.mjs`, `conformance-ref` matches the cases `conformance.json` pins;
   - a post under `content/news/` announces the release.
