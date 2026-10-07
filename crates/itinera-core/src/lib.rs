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
    AbortReason, DecidingHook, Event, EventBody, FailureCause, GiveUpCause, GiveUpHook, HookSource,
    JourneyAbort, JourneyFailure, JourneyId, LastFailure, Lifecycle, MissingData, Reason,
    RequestSource, Requester, RetryCause, Source, StepAttempt, StepHook, WorkflowHook,
};
#[cfg(feature = "unstable")]
pub use value::{AnyValue, Value};
