//! What a step or hook emitted about its own work while a test recorded it, and the record both
//! recorders keep.

use super::Level;
use crate::error::Interrupted;
use crate::report::GlobalMetadata;
use crate::step::Reporting;
use crate::value::{AnyValue, Value};

/// An event a step or hook emitted about its own work while a test recorded it: its level, its
/// message and its data, without the step, hook or attempt a journey would stamp it with.
///
/// [`StepEventReporter`](crate::step::StepEventReporter) and
/// [`HookEventReporter`](crate::policy::HookEventReporter) keep one for each event emitted
/// through the reporters they lend. It is available with the `testing` feature.
///
/// # Examples
///
/// ```
/// use itinera::event::Level;
/// use itinera::step::StepEventReporter;
///
/// let mut events = StepEventReporter::default();
/// events.reporter().warning_with("the gateway is slow", 300_i64)?;
///
/// let emitted = &events.emitted()[0];
/// assert_eq!(emitted.level(), Level::Warning);
/// assert_eq!(emitted.message(), "the gateway is slow");
/// assert_eq!(emitted.data::<i64>(), Some(&300));
/// # Ok::<(), itinera::error::Interrupted>(())
/// ```
#[derive(Clone, Debug)]
pub struct Emitted {
    level: Level,
    message: String,
    data: Option<AnyValue>,
}

impl Emitted {
    /// The event's level.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::event::{Emitted, Level};
    ///
    /// fn is_error(emitted: &Emitted) -> bool {
    ///     emitted.level() == Level::Error
    /// }
    /// ```
    #[must_use]
    pub fn level(&self) -> Level {
        self.level
    }

    /// The event's message.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::event::Emitted;
    ///
    /// fn says_slow(emitted: &Emitted) -> bool {
    ///     emitted.message().contains("slow")
    /// }
    /// ```
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The event's data, if it has data of type `T` exactly.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::event::Emitted;
    ///
    /// fn millis(emitted: &Emitted) -> Option<u64> {
    ///     emitted.data::<u64>().copied()
    /// }
    /// ```
    #[must_use]
    pub fn data<T: Value>(&self) -> Option<&T> {
        self.data.as_ref()?.downcast_ref()
    }
}

/// What a recorder of a step's or hook's events keeps: what was emitted, the global metadata
/// that was set, and whether a reporter failed, after which every call is interrupted.
#[derive(Debug, Default)]
pub(crate) struct Recorded {
    emitted: Vec<Emitted>,
    global_metadata: GlobalMetadata,
    interrupted: bool,
}

impl Recorded {
    /// A record whose reporters fail at once, so that every call is interrupted.
    pub(crate) fn failing() -> Self {
        Self {
            interrupted: true,
            ..Self::default()
        }
    }

    /// Lends a means of emitting into this record.
    pub(crate) fn reporting(&mut self) -> Reporting<'_> {
        Reporting::Recorded(self)
    }

    pub(crate) fn emitted(&self) -> &[Emitted] {
        &self.emitted
    }

    pub(crate) fn global_metadata(&self) -> &GlobalMetadata {
        &self.global_metadata
    }

    /// Keeps an event of this level, unless a reporter failed.
    pub(crate) fn record(
        &mut self,
        level: Level,
        message: String,
        data: Option<AnyValue>,
    ) -> Result<(), Interrupted> {
        self.interrupt_if_failed()?;
        self.emitted.push(Emitted {
            level,
            message,
            data,
        });
        Ok(())
    }

    /// Keeps an entry of global metadata, unless a reporter failed. As in a journey, the journey
    /// ID's own key interrupts the call and every later one.
    pub(crate) fn set(&mut self, key: String, value: AnyValue) -> Result<(), Interrupted> {
        self.interrupted |= key == GlobalMetadata::JOURNEY_ID;
        self.interrupt_if_failed()?;
        self.global_metadata.insert(key, value);
        Ok(())
    }

    fn interrupt_if_failed(&self) -> Result<(), Interrupted> {
        if self.interrupted {
            Err(Interrupted)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::StepReporter;

    fn reporter(recorded: &mut Recorded) -> StepReporter<'_> {
        StepReporter::from(recorded.reporting())
    }

    fn levels(recorded: &Recorded) -> Vec<Level> {
        recorded.emitted().iter().map(Emitted::level).collect()
    }

    #[test]
    fn what_is_emitted_is_kept_in_order_with_its_level_message_and_data() {
        let mut recorded = Recorded::default();
        let mut reporter = reporter(&mut recorded);

        reporter.info("charging").unwrap();
        reporter.warning_with("slow", 300_i64).unwrap();
        reporter.error("no receipt").unwrap();

        assert_eq!(
            levels(&recorded),
            [Level::Info, Level::Warning, Level::Error]
        );
        let slow = recorded.emitted().get(1).unwrap();
        assert_eq!(slow.message(), "slow");
        assert_eq!(slow.data::<i64>(), Some(&300));
        assert_eq!(slow.data::<String>(), None);
    }

    #[test]
    fn a_failing_record_interrupts_every_call_and_keeps_nothing() {
        let mut recorded = Recorded::failing();
        let mut reporter = reporter(&mut recorded);

        assert!(reporter.info("charging").is_err());
        assert!(reporter.set_global_metadata("user_id", 7_i64).is_err());

        assert!(recorded.emitted().is_empty());
        assert!(recorded.global_metadata().get("user_id").is_none());
    }

    #[test]
    fn setting_the_journey_ids_key_interrupts_that_call_and_every_later_one() {
        let mut recorded = Recorded::default();
        let mut reporter = reporter(&mut recorded);

        reporter.set_global_metadata("user_id", 7_i64).unwrap();
        assert!(
            reporter
                .set_global_metadata(GlobalMetadata::JOURNEY_ID, "J-1".to_string())
                .is_err()
        );
        assert!(reporter.info("charging").is_err());

        assert!(recorded.emitted().is_empty());
        assert!(recorded.global_metadata().get("user_id").is_some());
        assert!(
            recorded
                .global_metadata()
                .get(GlobalMetadata::JOURNEY_ID)
                .is_none()
        );
    }
}
