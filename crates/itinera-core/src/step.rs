//! Steps: the business units of a workflow, what they need, how they are built and run, their
//! attempts, their outcomes and the reasons they give.

use std::future::Future;
use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::pin::Pin;

use crate::error::Error;
use crate::mode::Synchronous;
use crate::policy::StepPolicyDescriptor;
use crate::value::{AnyValue, Value};

#[cfg(feature = "async")]
mod asynchronous;
mod needs;
mod outcome;
mod reporter;

#[cfg(feature = "async")]
pub use asynchronous::{AsyncStep, AsyncStepFactory, AsyncStepReporter};
pub(crate) use needs::{Got, InputNeed, Requirement};
pub use needs::{Input, OptionalInput, Resolved, StepNeeds};
pub use outcome::Outcome;
pub(crate) use outcome::OutcomeKind;
pub use reporter::StepReporter;
pub(crate) use reporter::{Level, Reporting};

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

/// What a synchronous workflow holds to build a step for each attempt: the step declares what it
/// needs, and the factory builds it from what was resolved.
///
/// A closure that returns `Result<Outcome, Error>` is a factory of a step that needs nothing.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::step::{Input, Outcome, Resolved, Step, StepFactory, StepNeeds};
///
/// struct Charge {
///     amount: i64,
/// }
///
/// impl Step for Charge {
///     fn run(self) -> Result<Outcome, Error> {
///         Ok(Outcome::success())
///     }
/// }
///
/// struct ChargeFactory;
///
/// const AMOUNT: Input<i64> = Input::new("amount");
///
/// impl StepFactory for ChargeFactory {
///     type Step<'a> = Charge;
///
///     fn needs(&self) -> StepNeeds {
///         StepNeeds::new().input(&AMOUNT)
///     }
///
///     fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Charge, Error> {
///         Ok(Charge {
///             amount: got.input(&AMOUNT)?,
///         })
///     }
/// }
/// ```
pub trait StepFactory: Send + Sync + 'static {
    /// The step it builds, which may borrow its attempt.
    type Step<'a>: Step;

    /// What the step needs. It is asked once, when the step descriptor is made.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{StepFactory, StepNeeds};
    ///
    /// fn needs(factory: &impl StepFactory) -> StepNeeds {
    ///     factory.needs()
    /// }
    /// ```
    fn needs(&self) -> StepNeeds;

    /// Builds the step for one attempt, from what was resolved for it.
    ///
    /// # Errors
    ///
    /// An error means the step could not be built, which aborts the journey with
    /// `step could not be built`. [`Interrupted`](crate::error::Interrupted), propagated from the
    /// step's reporter, is the exception: the journey was already aborted with `reporter failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::step::{Resolved, StepFactory};
    ///
    /// fn build<'a, F: StepFactory>(factory: &'a F, got: &mut Resolved<'a>) -> Result<F::Step<'a>, Error> {
    ///     factory.build(got)
    /// }
    /// ```
    fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Self::Step<'a>, Error>;
}

impl<F> StepFactory for F
where
    F: Fn() -> Result<Outcome, Error> + Send + Sync + 'static,
{
    type Step<'a> = &'a F;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new()
    }

    fn build<'a>(&'a self, _: &mut Resolved<'a>) -> Result<&'a F, Error> {
        Ok(self)
    }
}

/// A step being run: its outcome, or the error that ended it abnormally, once it finishes.
pub(crate) type Running<'a> = Pin<Box<dyn Future<Output = Result<Outcome, Error>> + Send + 'a>>;

/// A step factory of either mode, as a step descriptor holds it.
pub(crate) trait Attempts: Send + Sync {
    /// Builds the step for one attempt and starts it, or fails to build it.
    fn attempt<'a>(&'a self, got: Got<'a>) -> Result<Running<'a>, Error>;
}

/// A synchronous step factory, whose steps run as soon as they are built.
struct Synchronously<F> {
    factory: F,
}

impl<F: StepFactory> Attempts for Synchronously<F> {
    fn attempt<'a>(&'a self, got: Got<'a>) -> Result<Running<'a>, Error> {
        let step = self.factory.build(&mut Resolved::new(got))?;
        Ok(Box::pin(std::future::ready(step.run())))
    }
}

/// What a workflow holds for one step: its name, what it needs, how to build it for each
/// attempt, and the step policies attached to it, in the order they were attached.
///
/// `M` is the mode of the workflows that may hold it: a step of a synchronous workflow comes from
/// a [`StepFactory`].
///
/// # Examples
///
/// ```
/// use itinera::policy::{StepHook, StepPolicyDescriptor};
/// use itinera::step::{Outcome, StepDescriptor, step_name};
///
/// let audit = StepPolicyDescriptor::new("audit", StepHook::OnStepSuccess);
/// let charge = StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())).policy(audit);
/// assert_eq!(charge.name().to_string(), "charge");
/// ```
#[derive(derive_more::Debug)]
pub struct StepDescriptor<M = Synchronous> {
    name: StepName,
    needs: StepNeeds,
    #[debug(skip)]
    factory: Box<dyn Attempts>,
    policies: Vec<StepPolicyDescriptor>,
    #[debug(skip)]
    mode: PhantomData<M>,
}

impl StepDescriptor {
    /// Describes a step of a synchronous workflow with its name and its factory, with no
    /// policies.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    ///
    /// let ship = StepDescriptor::new(step_name!("ship"), || Ok(Outcome::success()));
    /// ```
    pub fn new(name: StepName, factory: impl StepFactory) -> Self {
        Self::holding(name, factory.needs(), Box::new(Synchronously { factory }))
    }
}

impl<M> StepDescriptor<M> {
    fn holding(name: StepName, needs: StepNeeds, factory: Box<dyn Attempts>) -> Self {
        Self {
            name,
            needs,
            factory,
            policies: Vec::new(),
            mode: PhantomData,
        }
    }

    /// Attaches a step policy, after those already attached.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{StepHook, StepPolicyDescriptor};
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    ///
    /// let audit = StepPolicyDescriptor::new("audit", StepHook::OnStepSuccess);
    /// let alarm = StepPolicyDescriptor::new("alarm", StepHook::OnStepFailure);
    /// let charge = StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success()))
    ///     .policy(audit)
    ///     .policy(alarm);
    /// ```
    pub fn policy(mut self, policy: StepPolicyDescriptor) -> Self {
        self.policies.push(policy);
        self
    }

    /// The step's name.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    ///
    /// let ship = StepDescriptor::new(step_name!("ship"), || Ok(Outcome::success()));
    /// assert_eq!(ship.name(), step_name!("ship"));
    /// ```
    pub fn name(&self) -> StepName {
        self.name
    }

    pub(crate) fn needs(&self) -> &StepNeeds {
        &self.needs
    }

    pub(crate) fn policies(&self) -> &[StepPolicyDescriptor] {
        &self.policies
    }

    /// Builds the step for one attempt and starts it, or fails to build it.
    pub(crate) fn attempt<'a>(&'a self, got: Got<'a>) -> Result<Running<'a>, Error> {
        self.factory.attempt(got)
    }

    pub(crate) fn is_named(&self, name: StepName) -> bool {
        self.name == name
    }
}

/// A step and one of its attempts, counted from 1.
///
/// # Examples
///
/// ```
/// use itinera::step::StepAttempt;
///
/// fn describe(attempt: &StepAttempt) -> String {
///     format!("{}, attempt {}", attempt.step, attempt.attempt)
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct StepAttempt {
    /// The step this attempt belongs to.
    pub step: StepName,
    /// The attempt number, from 1.
    pub attempt: NonZeroU32,
}

impl StepAttempt {
    /// The first attempt of a step.
    pub(crate) fn first(step: StepName) -> Self {
        Self {
            step,
            attempt: NonZeroU32::MIN,
        }
    }
}

/// Why a step failed or was skipped, or why a hook failed the journey: a code, an optional
/// message and optional details.
///
/// The details are a [`Value`], captured when the reason is made.
///
/// # Examples
///
/// ```
/// use itinera::step::Reason;
///
/// let reason = Reason::new("card-declined")
///     .with_message("the card was declined")
///     .with_details(3_i64);
/// assert_eq!(reason.code(), "card-declined");
/// assert_eq!(reason.message(), Some("the card was declined"));
/// assert_eq!(reason.details().and_then(|d| d.downcast_ref::<i64>()), Some(&3));
/// ```
#[derive(Clone, Debug)]
pub struct Reason {
    code: String,
    message: Option<String>,
    details: Option<AnyValue>,
}
impl Reason {
    /// Makes a reason with a code, and no message or details.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::Reason;
    ///
    /// let reason = Reason::new("out-of-stock");
    /// assert_eq!(reason.code(), "out-of-stock");
    /// assert!(reason.message().is_none());
    /// assert!(reason.details().is_none());
    /// ```
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: None,
            details: None,
        }
    }

    /// Gives the reason a message.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::Reason;
    ///
    /// let reason = Reason::new("out-of-stock").with_message("none left");
    /// assert_eq!(reason.message(), Some("none left"));
    /// ```
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Gives the reason details, which must be a [`Value`].
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::Reason;
    ///
    /// let reason = Reason::new("out-of-stock").with_details(vec!["SKU-1".to_string()]);
    /// assert!(reason.details().is_some());
    /// ```
    pub fn with_details<T: Value>(mut self, details: T) -> Self {
        self.details = Some(AnyValue::new(details));
        self
    }

    /// The reason's code.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::Reason;
    ///
    /// assert_eq!(Reason::new("late").code(), "late");
    /// ```
    pub fn code(&self) -> &str {
        &self.code
    }

    /// The reason's message, if it has one.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::Reason;
    ///
    /// assert_eq!(Reason::new("late").message(), None);
    /// ```
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// The reason's details, if it has any.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::Reason;
    ///
    /// let reason = Reason::new("late").with_details(true);
    /// assert_eq!(reason.details().and_then(|d| d.downcast_ref::<bool>()), Some(&true));
    /// ```
    pub fn details(&self) -> Option<&AnyValue> {
        self.details.as_ref()
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
