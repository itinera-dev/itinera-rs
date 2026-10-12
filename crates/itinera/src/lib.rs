//! Itinera is a workflow framework that keeps business rules separate from flow control.
//!
//! Its behaviour is defined by the [Itinera specification](https://github.com/itinera-dev/spec),
//! whose vocabulary this crate uses.
//!
//! # The parts of a workflow
//!
//! - A **step** holds one business rule. It declares the data it needs, which the workflow
//!   supplies, runs once per attempt, and reports an outcome: success, failure or skipped. Only an
//!   attempt that succeeds commits what the step contributed. See [`step`].
//! - A **policy** holds flow control. Its hooks, such as `on step failure` or `on workflow
//!   success`, are called in response to a step's outcome or at the end of the journey, and may
//!   end the journey early. A hook may use a role, an interface the workflow provides. See
//!   [`policy`].
//! - A **workflow** lists its steps in the order they run, with their retry budgets and policies,
//!   its own policies, its input adapters, its ID generator and its reporters. It is declared once,
//!   before any journey, and a declaration that breaks a rule of the specification is refused. See
//!   [`workflow`].
//! - A **workflow instance** is created for each journey, with its initial data and its reporters,
//!   and an **executor** runs it to a **result**: the journey succeeded, failed or was aborted.
//!   See [`instance`], [`executor`] and [`journey`].
//! - Everything that happens in a journey is an **event**, which the journey's reporters receive.
//!   See [`event`] and [`report`].
//!
#![cfg_attr(feature = "macros", doc = include_str!("../docs/first-workflow.md"))]
//!
//! # Features
//!
//! - `macros`, on by default: the attributes and derives that declare steps, policies, workflows
//!   and journey data. They expand to builder calls and implementations of public traits, which
//!   an application can write by hand.
//! - `async`: the asynchronous executor, and asynchronous steps, hooks, reporters and
//!   dispatchers.
//! - `testing`: `report::RecordingReporter`, a reporter that keeps every event for a test to
//!   read; enable it for tests only, under `[dev-dependencies]`.
//! - `unstable`: API the specification has not settled yet; empty for now.

#![forbid(unsafe_code)]

pub use itinera_core::*;
