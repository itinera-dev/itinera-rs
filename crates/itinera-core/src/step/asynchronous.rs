use std::future::Future;

use super::{Attempts, Got, Outcome, Resolved, Running, StepDescriptor, StepName};
use crate::error::Error;
use crate::mode::Asynchronous;

mod factory;
mod reporter;

pub use factory::AsyncStepFactory;
pub use reporter::AsyncStepReporter;

/// A step of an asynchronous workflow: built for one attempt, it does its work when run, and
/// reports an [`Outcome`].
///
/// Everything [`Step`](super::Step) says applies to it. A step whose work is all synchronous
/// still implements this trait, with a body that never awaits.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::step::{AsyncStep, Outcome};
///
/// struct Ship;
///
/// impl AsyncStep for Ship {
///     async fn run(self) -> Result<Outcome, Error> {
///         Ok(Outcome::success())
///     }
/// }
/// ```
pub trait AsyncStep: Send {
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
    /// use itinera::step::{AsyncStep, Outcome};
    ///
    /// async fn outcome_of(step: impl AsyncStep) -> Result<Outcome, Error> {
    ///     step.run().await
    /// }
    /// ```
    fn run(self) -> impl Future<Output = Result<Outcome, Error>> + Send;
}

impl<F, R> AsyncStep for &F
where
    F: Fn() -> R + Sync,
    R: Future<Output = Result<Outcome, Error>> + Send,
{
    fn run(self) -> impl Future<Output = Result<Outcome, Error>> + Send {
        self()
    }
}

/// An asynchronous step factory, whose steps are awaited.
struct Asynchronously<F> {
    factory: F,
}

impl<F: AsyncStepFactory> Attempts for Asynchronously<F> {
    fn attempt<'a>(&'a self, got: Got<'a>) -> Result<Running<'a>, Error> {
        let step = self.factory.build(&mut Resolved::new(got))?;
        Ok(Box::pin(step.run()))
    }
}

impl<W> StepDescriptor<W, Asynchronous> {
    /// Describes a step of an asynchronous workflow with its name and its factory, with no
    /// policies.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    ///
    /// struct Orders;
    ///
    /// let ship = StepDescriptor::<Orders, _>::new_async(step_name!("ship"), async || {
    ///     Ok(Outcome::success())
    /// });
    /// ```
    pub fn new_async(name: StepName, factory: impl AsyncStepFactory) -> Self {
        Self::holding(name, factory.needs(), Box::new(Asynchronously { factory }))
    }
}

impl<'a> Resolved<'a, Asynchronous> {
    /// Takes the step's reporter.
    ///
    /// # Errors
    ///
    /// When the step's needs do not declare a reporter, or it was already taken. Building fails
    /// with that error, and the journey is aborted with `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::step::{AsyncStepReporter, Resolved};
    ///
    /// fn reporter<'a>(
    ///     got: &mut Resolved<'a, Asynchronous>,
    /// ) -> Result<AsyncStepReporter<'a>, Error> {
    ///     got.reporter()
    /// }
    /// ```
    pub fn reporter(&mut self) -> Result<AsyncStepReporter<'a>, Error> {
        self.reporting().map(AsyncStepReporter::from)
    }
}
