//! The core of Itinera: values and the data bag, events, reporters and dispatchers, steps,
//! policies, execution modes, workflow descriptors, workflow instances and the local executors.
//!
//! Applications depend on the `itinera` crate, which re-exports this one.

#![forbid(unsafe_code)]

pub mod error;
pub mod event;
pub mod executor;
pub mod instance;
pub mod journey;
pub mod mode;
pub mod policy;
pub mod report;
pub mod step;
pub mod value;
pub mod workflow;

mod engine;
