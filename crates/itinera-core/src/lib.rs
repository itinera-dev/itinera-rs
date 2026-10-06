//! The core of Itinera: values and the data bag, events, reporters and dispatchers, steps,
//! policies, workflow descriptors, workflow instances and the local executors.
//!
//! Applications depend on the `itinera` crate, which re-exports this one.

#![forbid(unsafe_code)]

#[cfg_attr(not(feature = "unstable"), allow(dead_code, unreachable_pub))]
mod error;
#[cfg_attr(not(feature = "unstable"), allow(dead_code, unreachable_pub))]
mod event;
#[cfg_attr(not(feature = "unstable"), allow(dead_code, unreachable_pub))]
mod value;

#[cfg(feature = "unstable")]
pub use error::Error;
#[cfg(feature = "unstable")]
pub use event::{
    AbortDetails, AbortReason, DecidedBy, Event, EventBody, Failure, GiveUpCause, HookName,
    HookRef, HookSource, JourneyId, Lifecycle, Reason, Requester, RetryCause, Source, StepAttempt,
};
#[cfg(feature = "unstable")]
pub use value::{AnyValue, Value};
