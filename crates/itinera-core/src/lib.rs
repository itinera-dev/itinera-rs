//! The core of Itinera: values and the data bag, events, reporters and dispatchers, steps,
//! policies, execution modes, workflow descriptors, workflow instances and the local executors.
//!
//! Applications depend on the `itinera` crate, which re-exports this one.

#![forbid(unsafe_code)]

#[cfg(feature = "unstable")]
pub mod error;
#[cfg(not(feature = "unstable"))]
#[allow(dead_code, unreachable_pub, unused_imports)]
mod error;

#[cfg(feature = "unstable")]
pub mod value;
#[cfg(not(feature = "unstable"))]
#[allow(dead_code, unreachable_pub, unused_imports)]
mod value;

#[cfg(feature = "unstable")]
pub mod journey;
#[cfg(not(feature = "unstable"))]
#[allow(dead_code, unreachable_pub, unused_imports)]
mod journey;

#[cfg(feature = "unstable")]
pub mod step;
#[cfg(not(feature = "unstable"))]
#[allow(dead_code, unreachable_pub, unused_imports)]
mod step;

#[cfg(feature = "unstable")]
pub mod policy;
#[cfg(not(feature = "unstable"))]
#[allow(dead_code, unreachable_pub, unused_imports)]
mod policy;

#[cfg(feature = "unstable")]
pub mod event;
#[cfg(not(feature = "unstable"))]
#[allow(dead_code, unreachable_pub, unused_imports)]
mod event;

#[cfg(feature = "unstable")]
pub mod report;
#[cfg(not(feature = "unstable"))]
#[allow(dead_code, unreachable_pub, unused_imports)]
mod report;

#[cfg(feature = "unstable")]
pub mod mode;
#[cfg(not(feature = "unstable"))]
#[allow(dead_code, unreachable_pub, unused_imports)]
mod mode;

#[cfg(feature = "unstable")]
pub mod workflow;
#[cfg(not(feature = "unstable"))]
#[allow(dead_code, unreachable_pub, unused_imports)]
mod workflow;

#[cfg(feature = "unstable")]
pub mod instance;
#[cfg(not(feature = "unstable"))]
#[allow(dead_code, unreachable_pub, unused_imports)]
mod instance;

#[cfg(feature = "unstable")]
pub mod executor;
#[cfg(not(feature = "unstable"))]
#[allow(dead_code, unreachable_pub, unused_imports)]
mod executor;

mod engine;
