---
name: architecture
description: How Claude finds which parts of docs/architecture.md in itinera-rs a change makes untrue, and checks the whole document against the code. Use when reviewing a change, when a stage or tier lands, and when asked whether the architecture diagrams are still true.
---

# Keeping the architecture document true

[AGENTS.md](../../../AGENTS.md) holds the rule, in the bullet on `docs/architecture.md`: a change that makes the document untrue updates it in the same pull request. This skill says how to tell which parts a change touches, and how to check them. It adds no rule of its own.

## What each section describes

Each section of a tier ends with a "Where it lives" line. The table below repeats those places, and adds what else makes the section untrue when it changes.

| Section | The code it describes | Also untrue when |
|---|---|---|
| The crates | `Cargo.toml` of each crate, `crates/itinera/src/lib.rs`, `crates/itinera-core/src/lib.rs` | a crate, a feature or a public module of `itinera-core` is added, removed or renamed |
| What a workflow is made of | `workflow/builder.rs`, `step/descriptor.rs`, `policy/descriptor/`, `workflow/input_adapter.rs`, `service/descriptor.rs`, `instance.rs`, `instance/` | a descriptor gains or loses a part |
| How the macros go into the mix | `crates/itinera-macros/src/`, `step/described.rs`, `step/factory.rs`, `step/from_inputs.rs`, `step/descriptor.rs`, `step/rescue/described.rs`, `step/rescue/factory.rs`, `step/rescue/from_requests.rs`, `policy/described.rs`, `policy/role.rs`, `workflow/described.rs`, `journey/initial_data.rs`, `journey/output.rs` | a macro is added, or generates another trait or call |
| From declaration to result | `workflow/builder.rs`, `instance/builder.rs`, `executor.rs`, `executor/started.rs`, `executor/refusal.rs`, `executor/asynchronous.rs`, `executor/local_executor_for.rs`, `executor/local_executor_for/dispatcher_factory.rs`, `mode/has_mode.rs`, `run` and `finish` in `engine.rs`, `introduce` in `engine/metadata.rs` | a refusal, an instance error or an executor is added |
| The executor loop | `travel`, `scan` and `run_step` in `engine/scan.rs`, `Next` in `engine.rs`, `engine/hooks/workflow.rs`, `engine/end.rs` | a way on from an attempt, or a workflow hook, is added |
| One attempt | `engine/attempt.rs`, `engine/attempt/policies.rs`, `engine/inputs.rs`, `engine/services.rs`, `engine/runner.rs`, `engine/script.rs`, `step/factory.rs`, `step/script.rs`, `step/ending.rs`, `instance/scripted.rs`, `instance/scripted/`, `journey/scripted_run.rs` and `journey/scripted_run/` | the order of what an attempt does changes |
| Parameter injection | `engine/inputs.rs`, `engine/request.rs`, `engine/services.rs`, `answer` in `engine/hooks.rs`, `engine/hooks/step/left.rs`, `rescue_request` in `engine/rescue.rs`, `policy/kind/`, `step/input.rs`, `step/input/`, `step/needs.rs`, `policy/needs/`, `service/token.rs` | a requester, a kind of request or a source of values is added |
| Outcomes | `engine/outcome.rs`, `engine/hooks/step.rs`, `step/outcome.rs`, `step/rescue/outcome.rs` | an outcome or a step hook is added |
| The decision | `decide` in `engine/decision.rs`, `engine/retry.rs`, `workflow/violation/admission.rs` | a decision or a give-up cause is added |
| Rescue | `engine/rescue.rs`, `step/rescue/`, `workflow/violation/admission.rs` | what a rescue step may request or attach changes |
| Policies and hooks | `call_hook` in `engine/hooks.rs`, `engine/hooks/step.rs`, `engine/hooks/workflow.rs`, `policy/`, `policy/lifecycle.rs`, `workflow/violation/admission.rs` | a hook, a lifecycle or when policies are built changes |
| Events and delivery | `engine/emitter.rs`, `engine/metadata.rs`, `engine/guard.rs`, `engine/asynchronous.rs`, `report/`, `step/reporting.rs`, `event/` | an event, a handle or the delivery rules change |
| How a journey ends | `engine/end.rs`, `journey/result.rs`, `journey/abort.rs`, `journey/abort/`, `executor/refusal.rs` | a status, a failure, an abort reason or a refusal is added |

Paths under `engine/`, `step/` and the like are in `crates/itinera-core/src/`.

## Checking a change

1. List the files the diff changes, and the public names, events, aborts, failures, hooks, requests, features and modules it adds, removes or renames.
2. For each, find the sections of the table it touches.
3. For each section found, read its diagram, its numbered list, its tables and its "Where it lives" line against the code after the change. Each statement is either still true, or untrue, with the `file:line` that contradicts it.
4. Check that every path and name in a "Where it lives" line still exists.
5. For each untrue statement, check that the diff corrects it. A missing correction is a finding, with the text to write.
6. A change that touches no section needs nothing, and the review says so in one line.

## Checking the whole document

When a stage or a tier lands, or when asked, do step 3 and 4 for every section, against the current `main`. A new tier gets its own section, `## Tier N`, holding only what that tier adds or changes, with the same kinds of diagram.

## Writing a correction

1. Follow the "Writing" section of AGENTS.md: every diagram is Mermaid, with `accTitle` and `accDescr`, and the numbered list after it says everything the diagram shows. Change the diagram, its `accDescr` and its list together.
2. Use the specification's vocabulary, and the Rust names of the code.
3. Before pushing, render every changed diagram to be sure it parses, for example with `npx -y @mermaid-js/mermaid-cli -i diagram.mmd -o diagram.svg`. In a cloud session, point it at the pre-installed Chromium with a Puppeteer configuration file holding `{"executablePath": "/opt/pw-browsers/chromium", "args": ["--no-sandbox"]}`, passed with `-p`.
