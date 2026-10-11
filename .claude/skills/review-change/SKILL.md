---
name: review-change
description: How Claude has a change to itinera-rs reviewed by a separate reviewer agent, and the checklist that reviewer works through. Use before opening a pull request, after every later push to an open one, and when a reviewer's findings come back.
---

# Reviewing a change

[AGENTS.md](../../../AGENTS.md) is the rulebook. Its "Pull requests and stacks" section says when a change is reviewed and when a review passes, and "Working with the maintainer" says when to report. This skill adds only how Claude runs the review. It states no rule of its own: every item below points at the AGENTS.md bullet that is its criterion.

## Starting the reviewer

Start a separate agent with the most capable model available. Its prompt says:

1. It is read-only. It edits, commits and pushes nothing, and it never switches branches in the shared checkout. To build a revision, it adds a `git worktree` under the session's scratchpad directory, or under `/tmp` when there is none, with a target directory of its own, and removes the worktree afterwards.
2. Which commits to review, as revisions it can diff, what the change is meant to do, and the draft commit message and pull request description.
3. That AGENTS.md is binding, together with [docs/tier-1-plan.md](../../../docs/tier-1-plan.md), the tech specs under `docs/specs/` that the change touches, and the specification chapters behind them.
4. The checklist below, in full.
5. The report it returns: findings marked BLOCKING or NIT, each with `file:line` and a concrete fix, then a verdict, APPROVE or CHANGES NEEDED.

When findings have been fixed, resume the same reviewer with what changed, so that it keeps its context.

## The checklist

For each item, the reviewer lists every place in the diff where the item applies, not only the first, and checks each against the AGENTS.md bullet named in quotes.

**Where behaviour comes from**

1. Every behaviour the code adds, against "Behaviour comes only from the specification" and "If the specification has a gap". A gap or contradiction is reported, never decided in code.
2. `conformance.json`, against the paragraph after the list of stages: a proposal is added only by the pull request that completes it, and proofs of impossibility land with their feature.

**Code**

3. Formatting configuration, against "Formatting is plain `rustfmt`": no `rustfmt.toml` or `.rustfmt.toml`.
4. Every new name of a type, function, variant, field and module, against "Names use the specification's vocabulary exactly".
5. Every `unwrap`, `expect`, `panic!`, indexing, `todo!` and `unimplemented!` outside tests, against the rule that library code never panics while a journey runs; a panic allowed at declaration has `#[expect(clippy::panic, reason = "…")]` and a `# Panics` section.
6. Every hand-written trait implementation, helper function and type, against "Reuse before writing", "Error enums derive", and the bullet on `Debug` where it exists. For each, name what could provide it instead, and weigh it as that bullet says.
7. Every constraint stated in documentation, the specification or the plan for the new types, against "Types hold every constraint".
8. Every struct with one field, against "A struct with one field is a tuple struct", in both directions, including how code outside its module reaches the field.
9. Every new type's module, against "Modules follow concepts", "A module with submodules decides its public face" and "Every module is a file of its own".
10. Every file the diff adds or makes longer, against "A file past 200 lines holds one thing". Count its lines, documentation included and test module excluded. Past 200, list what it holds, and unless it is all one type, propose the private submodules it splits into.
11. Every closure, including closures bound with `let` before a chain, against "A chain of adapters reads as a sentence of names".
12. Every `for` loop, against "Iterating to get a result uses adapters". Asserting in a test acts on each item, and, in tests, so does a loop over every value of a closed set, as the bullet on several inputs says where it exists.
13. Every item's visibility, against "Everything is `pub(crate)`", and the `unstable` gate, against "Public API of a proposal not yet listed".
14. Every crate root, against "Every crate has `#![forbid(unsafe_code)]`".
15. Every public item and its example, against "Every public item has documentation" and "Documentation is written where an item is defined".
16. Every new or rewritten test, against "Test names state the rule", the bullet on several inputs where it exists, and "A chain that three or more tests". Check also that no coverage is lost.
17. Every test the diff adds or moves, and every test module it adds or makes longer, against "Unit tests live with the code they check". Name the module whose code decides what each test checks, and the module it is in. Count each test module's lines; past 150, it is a file of its own.
18. Every line of code, test and example, against "Every line does something". For each line that changes nothing, say so and ask for it to be removed.

**Comments**

19. Every comment, doc comment and identifier, against the "Comments" section.

**Pull requests and writing**

20. The commit message and pull request description, against "Every pull request names an open issue" and "Commit messages and pull request descriptions".
21. Every document, description and diagram, against the "Writing" section.

**Checks**

22. The checks under "Running the checks", run in the worktree at the reviewed revision. Report any that fail.

## Handling the findings

1. Verify each finding before acting on it. Fix the real ones, including nits that are plainly right.
2. A finding you reject goes back to the reviewer with your reason. A rejected BLOCKING finding is closed only when the resumed reviewer concedes it, or when the maintainer decides.
3. Run the checks, push, and resume the reviewer with what changed.
4. When you tell the maintainer a pull request is ready, mention the review.
