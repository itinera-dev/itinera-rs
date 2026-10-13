//! The record of one step's attempts in a scripted run.

use super::AttemptRecord;
use crate::step::StepName;

/// The record of one step, or rescue step, in a scripted run: every attempt handed to the
/// construction of its step, scripted or run for real, in order. [`ScriptedRun::step`] gives
/// it.
///
/// An attempt that ended before it was handed to the construction of its step, for example
/// because a required input was missing or a service provider failed, is not in it. One whose
/// step's constructor failed is.
///
/// [`ScriptedRun::step`]: super::ScriptedRun::step
///
/// # Examples
///
/// ```
/// use itinera::instance::WorkflowInstance;
/// use itinera::step::{Outcome, StepDescriptor, step_name};
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Orders;
///
/// let orders = WorkflowDescriptor::builder("orders")
///     .step(
///         StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success()))
///             .retry_budget(1),
///     )
///     .build()?;
/// let run = orders
///     .instance(Orders)
///     .create()?
///     .scripted()
///     .step("charge")
///     .will_fail_retriably("timeout")
///     .on_attempt(2)
///     .will_succeed()
///     .run()?;
/// let charge = run.step("charge");
/// assert_eq!(charge.attempts().len(), 2);
/// assert!(charge.attempt(2).is_some());
/// assert!(charge.attempt(3).is_none());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
pub struct StepRecord {
    name: StepName,
    /// Its attempts, attempt 1 first.
    attempts: Vec<AttemptRecord>,
}

impl StepRecord {
    /// The record of the step named `name`, with its attempts among `attempts`.
    pub(super) fn of(name: StepName, attempts: &[AttemptRecord]) -> Self {
        let attempts = attempts
            .iter()
            .filter(|attempt| attempt.belongs_to(name))
            .cloned()
            .collect();
        Self { name, attempts }
    }

    /// Every attempt handed to the construction of the step, attempt 1 first.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    ///     .build()?;
    /// let run = orders.instance(Orders).create()?.scripted().run()?;
    /// assert_eq!(run.step("charge").attempts().len(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn attempts(&self) -> &[AttemptRecord] {
        &self.attempts
    }

    /// The attempt numbered `attempt`, from 1, if it was handed to the construction of the step.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    ///     .build()?;
    /// let run = orders.instance(Orders).create()?.scripted().run()?;
    /// assert!(run.step("charge").attempt(1).is_some());
    /// assert!(run.step("charge").attempt(2).is_none());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn attempt(&self, attempt: u32) -> Option<&AttemptRecord> {
        let index = usize::try_from(attempt).ok()?.checked_sub(1)?;
        self.attempts.get(index)
    }

    pub(super) fn is(&self, name: &str) -> bool {
        self.name.is(name)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::instance::WorkflowInstance;
    use crate::step::{Outcome, StepDescriptor, StepName};
    use crate::workflow::fixtures::{Orders, orders};

    #[rstest]
    #[case::attempt_0(0, false)]
    #[case::attempt_1(1, true)]
    fn attempts_are_numbered_from_1(#[case] attempt: u32, #[case] recorded: bool) {
        let charge = StepDescriptor::new(StepName::new("charge"), || Ok(Outcome::success()));
        let run = orders()
            .step(charge)
            .build()
            .unwrap()
            .instance(Orders)
            .create()
            .unwrap()
            .scripted()
            .run()
            .unwrap();

        assert_eq!(run.step("charge").attempt(attempt).is_some(), recorded);
    }
}
