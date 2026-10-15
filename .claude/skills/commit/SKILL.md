---
name: commit
description: How Claude writes a commit message in itinera-rs, its Conventional Commits subject and the changelog entries it carries as trailers, and checks it before pushing. Use before every commit, and when the commit messages check fails.
---

# Writing a commit

AGENTS.md "Pull requests and stacks" sets the rules, and docs/contributing.md "Commit messages and the changelog" gives an example and says how git-cliff and the `commit messages` check read a message. This skill adds how Claude decides each part.

## The subject

1. Choose the type by what the commit does, not by the files it touches:
   - `feat` adds or changes what an application can use: public API, the macros, a feature of a crate;
   - `fix` puts right what an application meets that does not do what the specification, the documentation or the type says;
   - `refactor` changes code without changing what an application meets, `perf` makes it faster, `test` changes only tests and their snapshots, `docs` only documentation, `ci` the workflows, `build` the manifests, the toolchain or `deny.toml`, and `chore` anything else, such as a release commit or a moved pin;
   - `revert` undoes an earlier commit, naming it in the body.
2. Choose the scope as the module of the concept the change belongs to, as AGENTS.md "Modules follow concepts" names them, such as `step`, `journey`, `event` or `macros`, or an area such as `release`, `book` or `conformance`. Leave it out when the change spans several.
3. Add `!` when code written for the last release no longer compiles or behaves differently. `git show v<last version>:<file>` settles what the last release had. A change to something added since the last release breaks nothing that was released, so it takes no `!`.
4. Write the description in the imperative, starting in lower case: `give an event's kind a type of its own`.
5. A revert's subject is `revert: <the reverted commit's description>`, not the `Revert "…"` that git and GitHub write, which the check refuses. When the reverted commit carried entries, the revert carries the entries that say what an application meets once it is undone, or `Changelog: none` when the reverted commit was never released.

## The entries

1. Compare what an application meets after the commit with what it met in the last release, not with the commit before. Each difference is one entry.
2. Name the section by that difference: `Added` for something new, `Changed` for something that works differently or moved, `Fixed` for something put right, `Removed` for something gone.
3. Write each entry as `CHANGELOG.md` writes entries: by public path, such as `journey::Read::present`, saying what it is or does, and for a breaking change what replaces what is gone. One entry per line; a long entry stays one line.
4. When a commit changes something that an earlier commit since the last release added, and that commit's entry still describes it rightly enough, the commit has no entry of its own. When the earlier entry is now untrue, the commit adds the entry that says what is true now; the release pull request reconciles the two, as the `release` skill says.
5. A `feat`, `fix` or `!` commit with no entry says `Changelog: none`.
6. The trailers close the message, after the body and a blank line, in this order: the entries, `Changelog: none` when it applies, `Refs #N` or `Closes #N`, then the attribution lines. Nothing else follows them.

## Before pushing

1. Check the commits that are not pushed yet, from this repository, with the `commit-messages` action of itinera-dev/actions, as docs/contributing.md says: `PYTHONPATH=<actions> python3 <actions>/commit-messages/main.py <base>..HEAD`, where `<base>` is `origin/main`, or the branch of the layer below in a stack. A shallow clone of itinera-dev/actions at `v1` is enough. Never push before it passes.
2. When the commit carries entries, `git cliff --unreleased --tag v<next version> --strip all` shows them as the release will print them.
3. Fix a message before it is pushed: the last commit's with `git commit --amend`, and an earlier one's, with the whole new message written to a file in the scratchpad, with `GIT_SEQUENCE_EDITOR="perl -pi -e 's/^pick/reword/ if \$. == 1'" GIT_EDITOR="cp <message file>" git rebase -i <commit>~1`, which rewords `<commit>` without opening an editor.

## After pushing

1. A pushed message that the check accepts is not reworded, even when a review finds an entry wrong or missing. The next commit carries the entry that says what is true, or a `docs` commit with the corrected entry, and the release pull request reconciles the two, as the `release` skill says.
2. A pushed message that the check refuses must be reworded, since the check is required. On a branch with no layer above it, reword it as above and push with `git push --force-with-lease`. On a layer of a stack with layers above it, reword it, then rebase every layer above it by hand, bottom first, each onto the new head of the layer below, as the `stack` skill's "A layer outside the stack" does, running the checks and pushing each with `--force-with-lease`.
