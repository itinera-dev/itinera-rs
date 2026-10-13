//! A recorder for tests, which lends a hook's reporter and keeps what the hook emits through it.

use crate::event::{Emitted, Recorded};
use crate::policy::HookReporter;
use crate::report::GlobalMetadata;

/// A recorder for tests: it lends a [`HookReporter`] that a test passes to a hook, or to a
/// service provider, and keeps what it emits through it and the global metadata it sets, for the
/// test to read once the call returns.
///
/// Nothing is delivered anywhere, and no event is stamped with a policy, a hook or a step. Every
/// call succeeds unless the recorder was made with [`failing`](HookEventReporter::failing), or
/// the hook set the journey ID's own key, which interrupts that call and every later one, as in
/// a journey.
///
/// It is available with the `testing` feature, which an application enables under
/// `[dev-dependencies]`.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::policy::{HookEventReporter, HookReporter};
///
/// struct Notify;
///
/// impl Notify {
///     fn failed(&self, order: String, mut reporter: HookReporter<'_>) -> Result<(), Error> {
///         reporter.warning_with("the customer was told", order)?;
///         Ok(())
///     }
/// }
///
/// let mut events = HookEventReporter::default();
/// Notify.failed("A-2".to_string(), events.reporter())?;
///
/// assert_eq!(events.emitted()[0].message(), "the customer was told");
/// assert_eq!(events.emitted()[0].data::<String>().map(String::as_str), Some("A-2"));
/// # Ok::<(), Error>(())
/// ```
#[derive(Debug, Default)]
pub struct HookEventReporter {
    recorded: Recorded,
}

impl HookEventReporter {
    /// A recorder whose reporters behave as if a reporter had failed: every emit and every entry
    /// of global metadata returns [`Interrupted`](crate::error::Interrupted), and nothing is
    /// recorded.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::HookEventReporter;
    ///
    /// let mut events = HookEventReporter::failing();
    /// assert!(events.reporter().info("closing").is_err());
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
    /// use itinera::policy::HookEventReporter;
    ///
    /// let mut events = HookEventReporter::default();
    /// events.reporter().set_global_metadata("tenant", "acme".to_string())?;
    /// assert!(events.global_metadata().get("tenant").is_some());
    /// # Ok::<(), itinera::error::Interrupted>(())
    /// ```
    pub fn reporter(&mut self) -> HookReporter<'_> {
        self.recorded.reporting().into()
    }

    /// Lends an asynchronous hook's reporter that emits into this recorder.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::HookEventReporter;
    ///
    /// let mut events = HookEventReporter::default();
    /// futures::executor::block_on(events.async_reporter().warning("slow"))?;
    /// assert_eq!(events.emitted()[0].message(), "slow");
    /// # Ok::<(), itinera::error::Interrupted>(())
    /// ```
    #[cfg(feature = "async")]
    pub fn async_reporter(&mut self) -> crate::policy::AsyncHookReporter<'_> {
        self.recorded.reporting().into()
    }

    /// What was emitted through the reporters this recorder lent, in order.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::HookEventReporter;
    ///
    /// assert!(HookEventReporter::default().emitted().is_empty());
    /// ```
    #[must_use]
    pub fn emitted(&self) -> &[Emitted] {
        self.recorded.emitted()
    }

    /// The global metadata set through the reporters this recorder lent, each key with
    /// its latest value, in the order the keys were first set.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::HookEventReporter;
    ///
    /// assert!(HookEventReporter::default().global_metadata().get("tenant").is_none());
    /// ```
    #[must_use]
    pub fn global_metadata(&self) -> &GlobalMetadata {
        self.recorded.global_metadata()
    }
}
