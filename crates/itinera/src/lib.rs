//! Itinera is a workflow framework that keeps business rules separate from flow control.
//!
//! Its behaviour is defined by the [Itinera specification](https://github.com/itinera-dev/spec),
//! whose vocabulary this crate uses. This release implements tier 1 of specification 0.1.0,
//! with the capabilities `sync`, `async` and `testing`, and passes its conformance cases
//! `v0.1.0-rc.16`.
//!
//! To learn Itinera step by step, read
//! [the Itinera book for Rust](https://itinera-dev.org/book/rust/), where every example compiles
//! and runs. [The Itinera site](https://itinera-dev.org/) explains why Itinera exists. This
//! documentation is the reference for each item.
//!
//! # The parts of a workflow
//!
//! - A **step** holds one business rule. It declares the data it needs, which the workflow
//!   supplies, runs once per attempt, and reports an outcome: success, failure or skipped. Only an
//!   attempt that succeeds commits what the step contributed. A step may have a **rescue step**,
//!   which runs when the step fails or terminates abnormally and, once it succeeds, lets the step
//!   run again. See [`step`].
//! - A **service** is an object of operations a step uses to do its business, such as a payment
//!   gateway. The step requests it by key, and a service provider of the workflow makes it, for
//!   the journey or for each attempt. See [`service`].
//! - A **policy** holds flow control. Its hooks, such as `on step failure` or `on workflow
//!   success`, are called in response to a step's outcome or at the end of the journey, and may
//!   end the journey early. A hook may use a role, an interface the workflow provides. See
//!   [`policy`].
//! - A **workflow** lists its steps in the order they run, with their retry budgets and policies,
//!   its own policies, its input adapters, its service providers, its ID generator and its
//!   reporters. It is declared once, before any journey, and a declaration that breaks a rule of
//!   the specification is refused. See [`workflow`].
//! - A **workflow instance** is created for each journey, with its initial data, its reporters and
//!   any initial global metadata, and an **executor** runs it to a **result**: the journey
//!   succeeded, failed or was aborted. See [`instance`], [`executor`] and [`journey`].
//! - Everything that happens in a journey is an **event**, which the journey's reporters receive,
//!   together with the journey's **global metadata**: entries of a key and a value, apart from
//!   the data bag, starting with the journey ID. See [`event`] and [`report`].
//!
#![cfg_attr(feature = "macros", doc = include_str!("../docs/first-workflow.md"))]
//!
//! # Features
//!
//! - `macros`, on by default: the attributes and derives that declare steps, rescue steps,
//!   policies, workflows, roles and journey data. They expand to builder calls and
//!   implementations of public traits, which an application can write by hand.
//! - `async`: the asynchronous executor, and asynchronous steps, rescue steps, hooks, service
//!   providers, reporters and dispatchers, with `mode::async_trait` for services and roles whose
//!   methods are asynchronous.
//! - `testing`: test support. `report::RecordingReporter`, a reporter that keeps every event, and
//!   the latest value of each entry of global metadata, for a test to read; fakes in place of
//!   service providers; scripted runs; and recorders that lend a step, a hook or an input adapter
//!   its handles outside a journey. Enable it for tests only, under `[dev-dependencies]`.
//! - `unstable`: API the specification has not settled yet; empty for now.

#![forbid(unsafe_code)]

pub use itinera_core::*;
