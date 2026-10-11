//! The core of Itinera: values and the data bag, events, reporters and dispatchers, steps,
//! policies, execution modes, workflow descriptors, workflow instances and the local executors,
//! and, with the `unstable` feature, services and their providers.
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
#[cfg(feature = "unstable")]
pub mod service;
pub mod step;
pub mod value;
pub mod workflow;

mod engine;
