//! Steps: the business units of a workflow, their attempts, and the reasons they give.

use std::num::NonZeroU32;

use crate::policy::StepPolicyDescriptor;
use crate::value::{AnyValue, Value};

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

/// What a workflow holds for one step: the step's name, how to run it, and the step policies
/// attached to it, in the order they were attached.
///
/// The step is a stand-in that can only succeed, until steps can be declared in full: each
/// journey runs it once, and it succeeds.
///
/// # Examples
///
/// ```
/// use itinera::policy::{StepHook, StepPolicyDescriptor};
/// use itinera::step::{StepDescriptor, step_name};
///
/// let audit = StepPolicyDescriptor::new("audit", StepHook::OnStepSuccess);
/// let charge = StepDescriptor::new(step_name!("charge"), || {}).policy(audit);
/// assert_eq!(charge.name().to_string(), "charge");
/// ```
#[derive(derive_more::Debug)]
pub struct StepDescriptor {
    name: StepName,
    #[debug(skip)]
    run: Box<dyn Fn() + Send + Sync>,
    policies: Vec<StepPolicyDescriptor>,
}

impl StepDescriptor {
    /// Describes a step with its name and how to run it, with no policies.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{StepDescriptor, step_name};
    ///
    /// let ship = StepDescriptor::new(step_name!("ship"), || {});
    /// # drop(ship);
    /// ```
    pub fn new(name: StepName, run: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            name,
            run: Box::new(run),
            policies: Vec::new(),
        }
    }

    /// Attaches a step policy, after those already attached.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{StepHook, StepPolicyDescriptor};
    /// use itinera::step::{StepDescriptor, step_name};
    ///
    /// let audit = StepPolicyDescriptor::new("audit", StepHook::OnStepSuccess);
    /// let alarm = StepPolicyDescriptor::new("alarm", StepHook::OnStepFailure);
    /// let charge = StepDescriptor::new(step_name!("charge"), || {})
    ///     .policy(audit)
    ///     .policy(alarm);
    /// # drop(charge);
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
    /// use itinera::step::{StepDescriptor, step_name};
    ///
    /// let ship = StepDescriptor::new(step_name!("ship"), || {});
    /// assert_eq!(ship.name(), step_name!("ship"));
    /// ```
    pub fn name(&self) -> StepName {
        self.name
    }

    pub(crate) fn policies(&self) -> &[StepPolicyDescriptor] {
        &self.policies
    }

    pub(crate) fn run(&self) {
        (self.run)();
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
