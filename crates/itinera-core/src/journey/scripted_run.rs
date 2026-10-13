//! What a test's scripted run gives back.

use super::JourneyResult;

/// What a scripted run of a workflow instance gives back once its journey is over: the result an
/// executor would have given.
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
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub struct ScriptedRun {
    /// The journey's result, as an executor would have given it.
    pub result: JourneyResult,
}
