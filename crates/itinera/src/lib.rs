//! Itinera is a workflow framework that keeps business rules separate from flow control.
//!
//! Its behaviour is defined by the [Itinera specification](https://github.com/itinera-dev/spec).

#![forbid(unsafe_code)]

#[cfg(feature = "unstable")]
pub use itinera_core::*;
