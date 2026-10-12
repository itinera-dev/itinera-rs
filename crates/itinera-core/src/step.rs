//! Steps: the business units of a workflow, what they need, how they are built and run, their
//! attempts, their outcomes and the reasons they give, and the rescue steps that fix the cause of
//! a step's failure.

use crate::error::Error;

#[cfg(feature = "async")]
mod asynchronous;
mod attempt;
mod described;
mod descriptor;
mod factory;
mod from_inputs;
mod input;
mod name;
mod needs;
mod outcome;
mod reason;
mod reporter;
mod reporting;
mod rescue;
mod resolved;
mod slots;

#[cfg(feature = "async")]
pub use asynchronous::{AsyncStep, AsyncStepFactory, AsyncStepReporter};
pub use attempt::StepAttempt;
pub use described::DescribedStep;
pub use descriptor::StepDescriptor;
pub use factory::StepFactory;
use factory::{Attempts, Running};
pub use from_inputs::FromInputs;
pub use input::{Input, OptionalInput};
#[cfg(feature = "macros")]
pub use itinera_macros::{RescueStep, Step, rescue_step, step};
pub use name::{StepName, step_name};
pub(crate) use needs::ServiceNeed;
pub use needs::StepNeeds;
pub(crate) use needs::{InputNeed, Requirement, RescueRequest, StepRequest};
pub use outcome::Outcome;
pub(crate) use outcome::OutcomeKind;
pub use reason::Reason;
pub use reporter::StepReporter;
pub(crate) use reporting::{Level, Reporting};
#[cfg(feature = "async")]
pub use rescue::{AsyncRescueFactory, AsyncRescueStep};
pub use rescue::{
    DescribedRescueStep, FromRescueRequests, RescueFactory, RescueNeeds, RescueOutcome,
    RescueResolved, RescueStep,
};
pub(crate) use rescue::{FromRescued, Rescues};
pub(crate) use resolved::Got;
pub use resolved::Resolved;
pub(crate) use slots::Slots;

/// A step: built for one attempt, it does its work when run, and reports an [`Outcome`].
///
/// `run` takes the step by value, so nothing of it survives into another attempt. Returning an
/// error is an abnormal termination: an error the step did not anticipate. A failure the step
/// chose is `Ok(Outcome::failure(reason))`.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::step::{Outcome, Reason, Step};
///
/// struct Charge {
///     amount: i64,
/// }
///
/// impl Step for Charge {
///     fn run(self) -> Result<Outcome, Error> {
///         if self.amount > 0 {
///             Ok(Outcome::success())
///         } else {
///             Ok(Outcome::failure(Reason::new("nothing-to-charge")))
///         }
///     }
/// }
/// ```
pub trait Step {
    /// Does the step's work and reports its outcome.
    ///
    /// # Errors
    ///
    /// An error is an abnormal termination of the attempt, which never aborts the journey.
    /// [`Interrupted`](crate::error::Interrupted), propagated from the step's reporter, is the
    /// exception: the journey was already aborted with `reporter failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::step::{Outcome, Step};
    ///
    /// fn outcome_of(step: impl Step) -> Result<Outcome, Error> {
    ///     step.run()
    /// }
    /// ```
    fn run(self) -> Result<Outcome, Error>;
}

impl<F: Fn() -> Result<Outcome, Error>> Step for &F {
    fn run(self) -> Result<Outcome, Error> {
        self()
    }
}
