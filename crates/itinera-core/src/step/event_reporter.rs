//! A recorder for tests, which lends a step's reporter and keeps what the step emits through it.

use crate::event::{Emitted, Recorded};
use crate::report::GlobalMetadata;
use crate::step::StepReporter;

/// A recorder for tests: it lends a [`StepReporter`] that a test passes to a step's `run`, and
/// keeps what the step emits through it and the global metadata it sets, for the test to read
/// once `run` returns.
///
/// Nothing is delivered anywhere, and no event is stamped with a step or an attempt. Every call
/// succeeds unless the recorder was made with [`failing`](StepEventReporter::failing), or the
/// step set the journey ID's own key, which interrupts that call and every later one, as in a
/// journey.
///
/// It is available with the `testing` feature, which an application enables under
/// `[dev-dependencies]`.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::step::{Outcome, StepEventReporter, StepReporter};
///
/// struct Charge {
///     price: i64,
/// }
///
/// impl Charge {
///     fn run(self, mut reporter: StepReporter<'_>) -> Result<Outcome, Error> {
///         reporter.info_with("charging", self.price)?;
///         Ok(Outcome::success())
///     }
/// }
///
/// let mut events = StepEventReporter::default();
/// Charge { price: 40 }.run(events.reporter())?;
///
/// assert_eq!(events.emitted()[0].message(), "charging");
/// assert_eq!(events.emitted()[0].data::<i64>(), Some(&40));
/// # Ok::<(), Error>(())
/// ```
#[derive(Debug, Default)]
pub struct StepEventReporter {
    recorded: Recorded,
}

impl StepEventReporter {
    /// A recorder whose reporters behave as if a reporter had failed: every emit and every entry
    /// of global metadata returns [`Interrupted`](crate::error::Interrupted), and nothing is
    /// recorded.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::StepEventReporter;
    ///
    /// let mut events = StepEventReporter::failing();
    /// assert!(events.reporter().info("charging").is_err());
    /// assert!(events.emitted().is_empty());
    /// ```
    #[must_use]
    pub fn failing() -> Self {
        Self {
            recorded: Recorded::failing(),
        }
    }

    /// Lends a reporter that emits into this recorder.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::StepEventReporter;
    ///
    /// let mut events = StepEventReporter::default();
    /// events.reporter().set_global_metadata("user_id", "U-7".to_string())?;
    /// assert!(events.global_metadata().get("user_id").is_some());
    /// # Ok::<(), itinera::error::Interrupted>(())
    /// ```
    pub fn reporter(&mut self) -> StepReporter<'_> {
        self.recorded.reporting().into()
    }

    /// Lends an asynchronous step's reporter that emits into this recorder.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::StepEventReporter;
    ///
    /// let mut events = StepEventReporter::default();
    /// futures::executor::block_on(events.async_reporter().warning("slow"))?;
    /// assert_eq!(events.emitted()[0].message(), "slow");
    /// # Ok::<(), itinera::error::Interrupted>(())
    /// ```
    #[cfg(feature = "async")]
    pub fn async_reporter(&mut self) -> crate::step::AsyncStepReporter<'_> {
        self.recorded.reporting().into()
    }

    /// What the step emitted through the reporters this recorder lent, in order.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::StepEventReporter;
    ///
    /// assert!(StepEventReporter::default().emitted().is_empty());
    /// ```
    #[must_use]
    pub fn emitted(&self) -> &[Emitted] {
        self.recorded.emitted()
    }

    /// The global metadata the step set through the reporters this recorder lent, each key with
    /// its latest value, in the order the keys were first set.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::StepEventReporter;
    ///
    /// assert!(StepEventReporter::default().global_metadata().get("user_id").is_none());
    /// ```
    #[must_use]
    pub fn global_metadata(&self) -> &GlobalMetadata {
        self.recorded.global_metadata()
    }
}
