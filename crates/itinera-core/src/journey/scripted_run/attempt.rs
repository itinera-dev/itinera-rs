//! The record of one attempt in a scripted run.

use crate::journey::Read;
use crate::step::{StepAttempt, StepName};
use crate::value::{AnyValue, Value};

/// The record of one attempt of a step, or rescue step, in a scripted run: what its step was
/// built with. [`StepRecord::attempts`] and [`StepRecord::attempt`] give it.
///
/// The step itself is not kept, since it borrows its services only for its attempt.
///
/// [`StepRecord::attempts`]: super::StepRecord::attempts
/// [`StepRecord::attempt`]: super::StepRecord::attempt
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::instance::WorkflowInstance;
/// use itinera::journey::Read;
/// use itinera::step::{Input, Outcome, Resolved, Step, StepDescriptor, StepFactory, StepNeeds};
/// use itinera::step::step_name;
/// use itinera::workflow::WorkflowDescriptor;
///
/// const AMOUNT: Input<i64> = Input::new("amount");
///
/// struct Charge;
///
/// impl Step for Charge {
///     fn run(self) -> Result<Outcome, Error> {
///         Ok(Outcome::success())
///     }
/// }
///
/// struct ChargeFactory;
///
/// impl StepFactory for ChargeFactory {
///     type Step<'a> = Charge;
///
///     fn needs(&self) -> StepNeeds {
///         StepNeeds::new().input(&AMOUNT)
///     }
///
///     fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Charge, Error> {
///         got.input(&AMOUNT)?;
///         Ok(Charge)
///     }
/// }
///
/// struct Orders;
///
/// let orders = WorkflowDescriptor::builder("orders")
///     .step(StepDescriptor::new(step_name!("charge"), ChargeFactory))
///     .build()?;
/// let run = orders
///     .instance(Orders)
///     .data("amount", 40_i64)
///     .create()?
///     .scripted()
///     .step("charge")
///     .will_succeed()
///     .run()?;
/// let attempt = run.step("charge").attempt(1).ok_or("charge was not attempted")?;
/// assert_eq!(attempt.input::<i64>("amount"), Read::Present(40));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug)]
pub struct AttemptRecord {
    attempt: StepAttempt,
    /// The values the step was built with: its inputs, in the order it declares them, then, for a
    /// rescue step, the contributions it requested from the failed attempt.
    values: Vec<(&'static str, AnyValue)>,
}

impl AttemptRecord {
    pub(crate) fn new(attempt: StepAttempt, values: Vec<(&'static str, AnyValue)>) -> Self {
        Self { attempt, values }
    }

    /// What the step was built with under the key, read as a `T`: the input, after the step's
    /// input adapter and the data bag, or, for a rescue step, the contribution it requested from
    /// the failed attempt. An input with a value is read before a contribution under the same
    /// key. It is [`Read::Absent`] when the step had no value under the key, or did not request
    /// it.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::journey::Read;
    /// use itinera::step::{Input, Outcome, Resolved, Step, StepDescriptor, StepFactory, StepNeeds};
    /// use itinera::step::step_name;
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// const AMOUNT: Input<i64> = Input::new("amount");
    ///
    /// struct Charge;
    ///
    /// impl Step for Charge {
    ///     fn run(self) -> Result<Outcome, Error> {
    ///         Ok(Outcome::success())
    ///     }
    /// }
    ///
    /// struct ChargeFactory;
    ///
    /// impl StepFactory for ChargeFactory {
    ///     type Step<'a> = Charge;
    ///
    ///     fn needs(&self) -> StepNeeds {
    ///         StepNeeds::new().input(&AMOUNT)
    ///     }
    ///
    ///     fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Charge, Error> {
    ///         got.input(&AMOUNT)?;
    ///         Ok(Charge)
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), ChargeFactory))
    ///     .build()?;
    /// let run = orders
    ///     .instance(Orders)
    ///     .data("amount", 40_i64)
    ///     .create()?
    ///     .scripted()
    ///     .step("charge")
    ///     .will_succeed()
    ///     .run()?;
    /// let attempt = run.step("charge").attempt(1).ok_or("charge was not attempted")?;
    /// assert_eq!(attempt.input::<i64>("amount"), Read::Present(40));
    /// assert_eq!(attempt.input::<String>("amount"), Read::OtherType);
    /// assert_eq!(attempt.input::<i64>("discount"), Read::Absent);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn input<T: Value>(&self, key: &str) -> Read<T> {
        Read::of(self.values.iter().find_map(|value| value_under(value, key)))
    }

    pub(super) fn belongs_to(&self, step: StepName) -> bool {
        self.attempt.step == step
    }
}

fn value_under<'v>(
    (declared, value): &'v (&'static str, AnyValue),
    key: &str,
) -> Option<&'v AnyValue> {
    (*declared == key).then_some(value)
}
