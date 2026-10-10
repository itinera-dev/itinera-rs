use std::future::Future;

use super::{
    Attempts, Got, Level, Outcome, Reporting, Resolved, Running, StepDescriptor, StepName,
    StepNeeds,
};
use crate::error::{Error, Interrupted};
use crate::mode::Asynchronous;
use crate::value::{AnyValue, Value};

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

/// What an asynchronous workflow holds to build a step for each attempt: the step declares what
/// it needs, and the factory builds it from what was resolved.
///
/// A closure that returns a future of `Result<Outcome, Error>` is a factory of a step that needs
/// nothing.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Asynchronous;
/// use itinera::step::{AsyncStep, AsyncStepFactory, Input, Outcome, Resolved, StepNeeds};
///
/// struct Charge {
///     amount: i64,
/// }
///
/// impl AsyncStep for Charge {
///     async fn run(self) -> Result<Outcome, Error> {
///         Ok(Outcome::success())
///     }
/// }
///
/// struct ChargeFactory;
///
/// const AMOUNT: Input<i64> = Input::new("amount");
///
/// impl AsyncStepFactory for ChargeFactory {
///     type Step<'a> = Charge;
///
///     fn needs(&self) -> StepNeeds {
///         StepNeeds::new().input(&AMOUNT)
///     }
///
///     fn build<'a>(&'a self, got: &mut Resolved<'a, Asynchronous>) -> Result<Charge, Error> {
///         Ok(Charge {
///             amount: got.input(&AMOUNT)?,
///         })
///     }
/// }
/// ```
pub trait AsyncStepFactory: Send + Sync + 'static {
    /// The step it builds, which may borrow its attempt.
    type Step<'a>: AsyncStep;

    /// What the step needs. It is asked once, when the step descriptor is made.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{AsyncStepFactory, StepNeeds};
    ///
    /// fn needs(factory: &impl AsyncStepFactory) -> StepNeeds {
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
    /// use itinera::mode::Asynchronous;
    /// use itinera::step::{AsyncStepFactory, Resolved};
    ///
    /// fn build<'a, F: AsyncStepFactory>(
    ///     factory: &'a F,
    ///     got: &mut Resolved<'a, Asynchronous>,
    /// ) -> Result<F::Step<'a>, Error> {
    ///     factory.build(got)
    /// }
    /// ```
    fn build<'a>(&'a self, got: &mut Resolved<'a, Asynchronous>) -> Result<Self::Step<'a>, Error>;
}

impl<F, R> AsyncStepFactory for F
where
    F: Fn() -> R + Send + Sync + 'static,
    R: Future<Output = Result<Outcome, Error>> + Send,
{
    type Step<'a> = &'a F;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new()
    }

    fn build<'a>(&'a self, _: &mut Resolved<'a, Asynchronous>) -> Result<&'a F, Error> {
        Ok(self)
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

/// An asynchronous step's means of emitting its own events, `step_info`, `step_warning` and
/// `step_error`, each stamped with the step and its attempt.
///
/// Everything [`StepReporter`](super::StepReporter) says applies to it, except that each emit
/// call is awaited: the step waits there until every reporter has the event.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Asynchronous;
/// use itinera::step::{
///     AsyncStep, AsyncStepFactory, AsyncStepReporter, Outcome, Resolved, StepNeeds,
/// };
///
/// struct Charge<'a> {
///     reporter: AsyncStepReporter<'a>,
/// }
///
/// impl AsyncStep for Charge<'_> {
///     async fn run(mut self) -> Result<Outcome, Error> {
///         self.reporter.info_with("charging", 42_i64).await?;
///         Ok(Outcome::success())
///     }
/// }
///
/// struct ChargeFactory;
///
/// impl AsyncStepFactory for ChargeFactory {
///     type Step<'a> = Charge<'a>;
///
///     fn needs(&self) -> StepNeeds {
///         StepNeeds::new().reporter()
///     }
///
///     fn build<'a>(&'a self, got: &mut Resolved<'a, Asynchronous>) -> Result<Charge<'a>, Error> {
///         Ok(Charge {
///             reporter: got.reporter()?,
///         })
///     }
/// }
/// ```
#[derive(Debug, derive_more::From)]
pub struct AsyncStepReporter<'a> {
    reporting: Reporting<'a>,
}

impl AsyncStepReporter<'_> {
    /// Emits `step_info` with a message.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::step::AsyncStepReporter;
    ///
    /// async fn starting(reporter: &mut AsyncStepReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.info("starting").await
    /// }
    /// ```
    pub fn info(
        &mut self,
        message: impl Into<String>,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting.emit(Level::Info, message.into(), None)
    }

    /// Emits `step_info` with a message and data.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::step::AsyncStepReporter;
    ///
    /// async fn starting(reporter: &mut AsyncStepReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.info_with("starting", 42_i64).await
    /// }
    /// ```
    pub fn info_with<T: Value>(
        &mut self,
        message: impl Into<String>,
        data: T,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting
            .emit(Level::Info, message.into(), Some(AnyValue::new(data)))
    }

    /// Emits `step_warning` with a message.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::step::AsyncStepReporter;
    ///
    /// async fn slow(reporter: &mut AsyncStepReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.warning("the gateway is slow").await
    /// }
    /// ```
    pub fn warning(
        &mut self,
        message: impl Into<String>,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting.emit(Level::Warning, message.into(), None)
    }

    /// Emits `step_warning` with a message and data.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::step::AsyncStepReporter;
    ///
    /// async fn slow(reporter: &mut AsyncStepReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.warning_with("the gateway is slow", 42_i64).await
    /// }
    /// ```
    pub fn warning_with<T: Value>(
        &mut self,
        message: impl Into<String>,
        data: T,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting
            .emit(Level::Warning, message.into(), Some(AnyValue::new(data)))
    }

    /// Emits `step_error` with a message. It does not change the step's outcome.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::step::AsyncStepReporter;
    ///
    /// async fn no_receipt(reporter: &mut AsyncStepReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.error("the receipt was not printed").await
    /// }
    /// ```
    pub fn error(
        &mut self,
        message: impl Into<String>,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting.emit(Level::Error, message.into(), None)
    }

    /// Emits `step_error` with a message and data. It does not change the step's outcome.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::step::AsyncStepReporter;
    ///
    /// async fn no_receipt(reporter: &mut AsyncStepReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.error_with("the receipt was not printed", 42_i64).await
    /// }
    /// ```
    pub fn error_with<T: Value>(
        &mut self,
        message: impl Into<String>,
        data: T,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting
            .emit(Level::Error, message.into(), Some(AnyValue::new(data)))
    }
}
