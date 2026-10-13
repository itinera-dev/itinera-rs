//! What a test's scripted run gives back: the journey's result and what each attempt was built
//! with.

use super::JourneyResult;
use crate::step::StepName;

mod attempt;
mod step;

pub use attempt::AttemptRecord;
pub use step::StepRecord;

/// What a scripted run of a workflow instance gives back once its journey is over: the result an
/// executor would have given, and a record of every step's attempts.
///
/// It is available with the `testing` feature, which an application enables under
/// `[dev-dependencies]`.
///
/// # Examples
///
/// ```
/// use itinera::instance::WorkflowInstance;
/// use itinera::journey::StatusKind;
/// use itinera::step::{Outcome, StepDescriptor, step_name};
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Orders;
///
/// let orders = WorkflowDescriptor::builder("orders")
///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
///     .build()?;
/// let run = orders
///     .instance(Orders)
///     .create()?
///     .scripted()
///     .step("charge")
///     .will_fail("declined")
///     .run()?;
/// assert_eq!(run.result.status.kind(), StatusKind::Failed);
/// assert_eq!(run.step("charge").attempts().len(), 1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub struct ScriptedRun {
    /// The journey's result, as an executor would have given it.
    pub result: JourneyResult,
    /// Every step and rescue step of the workflow, in the order the workflow lists them, each
    /// rescue step after its step.
    steps: Vec<StepRecord>,
}

impl ScriptedRun {
    /// The run of the workflow's steps and rescue steps, named by `names`, with the attempts
    /// handed to their construction, in the order they were.
    pub(crate) fn new(
        result: JourneyResult,
        names: impl IntoIterator<Item = StepName>,
        attempts: &[AttemptRecord],
    ) -> Self {
        let steps = names
            .into_iter()
            .map(|name| StepRecord::of(name, attempts))
            .collect();
        Self { result, steps }
    }

    /// The record of the step, or rescue step, with this name: an empty one when the journey
    /// never reached it.
    ///
    /// # Panics
    ///
    /// When the workflow has no step or rescue step with this name.
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
    ///     .step(StepDescriptor::new(step_name!("ship"), || Ok(Outcome::success())))
    ///     .build()?;
    /// let run = orders
    ///     .instance(Orders)
    ///     .create()?
    ///     .scripted()
    ///     .step("charge")
    ///     .will_fail("declined")
    ///     .run()?;
    /// assert!(run.step("ship").attempts().is_empty());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[expect(
        clippy::panic,
        reason = "asking for a step the workflow does not have is a mistake in a test"
    )]
    pub fn step(&self, name: &str) -> &StepRecord {
        let Some(step) = self.steps.iter().find(|step| step.is(name)) else {
            panic!("the workflow has no step named {name}");
        };
        step
    }
}

#[cfg(test)]
mod tests {
    use super::ScriptedRun;
    use crate::instance::WorkflowInstance;
    use crate::step::{Outcome, RescueOutcome, StepDescriptor, StepName};
    use crate::workflow::fixtures::{Orders, orders};

    /// A run of the orders workflow whose charge step, which has a rescue step, succeeds.
    fn charged() -> ScriptedRun {
        let charge = StepDescriptor::new(StepName::new("charge"), || Ok(Outcome::success()))
            .rescue(|| Ok(RescueOutcome::success()));
        orders()
            .step(charge)
            .build()
            .unwrap()
            .instance(Orders)
            .create()
            .unwrap()
            .scripted()
            .step("charge")
            .will_succeed()
            .run()
            .unwrap()
    }

    #[test]
    fn a_rescue_step_the_journey_never_reached_has_a_record_without_attempts() {
        assert!(charged().step("charge_rescue").attempts().is_empty());
    }

    #[test]
    #[should_panic(expected = "the workflow has no step named chrage")]
    fn asking_for_a_step_the_workflow_does_not_have_panics() {
        charged().step("chrage");
    }
}
