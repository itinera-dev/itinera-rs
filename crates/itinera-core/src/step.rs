//! Steps: the business units of a workflow, what they need, how they are built and run, their
//! attempts, their outcomes and the reasons they give.

use crate::error::Error;

#[cfg(feature = "async")]
mod asynchronous;
mod attempt;
mod descriptor;
mod factory;
mod input;
mod needs;
mod outcome;
mod reason;
mod reporter;
mod reporting;
mod resolved;
mod slots;

#[cfg(feature = "async")]
pub use asynchronous::{AsyncStep, AsyncStepFactory, AsyncStepReporter};
pub use attempt::StepAttempt;
pub use descriptor::StepDescriptor;
pub use factory::StepFactory;
use factory::{Attempts, Running};
pub use input::{Input, OptionalInput};
pub use needs::StepNeeds;
pub(crate) use needs::{InputNeed, Requirement};
pub use outcome::Outcome;
pub(crate) use outcome::OutcomeKind;
pub use reason::Reason;
pub use reporter::StepReporter;
pub(crate) use reporting::{Level, Reporting};
pub(crate) use resolved::Got;
pub use resolved::Resolved;
pub(crate) use slots::Slots;

/// A step's name: non-empty text, fixed when the program is compiled, unique within its workflow
/// and compared case-sensitively.
///
/// [`step_name!`] makes one from a constant, and refuses an empty one at compile time.
/// [`StepName::new`] does the same from any `&'static str`, at compile time in a `const` context
/// and otherwise when it is called.
///
/// # Examples
///
/// ```
/// use itinera::step::{StepName, step_name};
///
/// let charge: StepName = step_name!("charge");
/// assert_eq!(charge.to_string(), "charge");
/// let text: &str = charge.as_ref();
/// assert_eq!(text, "charge");
/// ```
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    derive_more::Display,
    derive_more::Into,
    derive_more::AsRef,
)]
#[display("{name}")]
#[as_ref(forward)]
pub struct StepName {
    name: &'static str,
}

impl StepName {
    /// Makes a step name.
    ///
    /// # Panics
    ///
    /// Panics if the name is empty, which is a mistake in the workflow's declaration. In a
    /// `const` context, such as [`step_name!`], that is a compile error instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::StepName;
    ///
    /// const SHIP: StepName = StepName::new("ship");
    /// assert_eq!(SHIP.to_string(), "ship");
    /// ```
    #[expect(
        clippy::panic,
        reason = "an empty step name is a mistake in a declaration, before any journey"
    )]
    pub const fn new(name: &'static str) -> Self {
        if name.is_empty() {
            panic!("a step name cannot be empty");
        }
        Self { name }
    }
}

/// Makes a [`StepName`] from a constant, and refuses an empty one at compile time.
///
/// # Examples
///
/// ```
/// use itinera::step::step_name;
///
/// assert_eq!(step_name!("charge").to_string(), "charge");
/// ```
///
/// An empty name does not compile:
///
/// ```compile_fail,E0080
/// use itinera::step::step_name;
///
/// let nameless = step_name!("");
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __step_name {
    ($name:expr) => {
        const { $crate::step::StepName::new($name) }
    };
}

#[doc(inline)]
pub use crate::__step_name as step_name;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "a step name cannot be empty")]
    fn a_step_name_cannot_be_empty() {
        let _ = StepName::new("");
    }
}
