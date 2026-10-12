//! A reporter for tests, which keeps every event and the global metadata it receives.

use std::sync::{Arc, Mutex, PoisonError};

use crate::error::Error;
use crate::event::Event;
use crate::report::{GlobalMetadata, Reporter};
use crate::value::AnyValue;

/// A reporter for tests: it keeps every event it receives, and the latest value of each entry of
/// global metadata, for the test to read once the journey is over.
///
/// Its clones share what they keep. A test makes one, adds a clone to the workflow instance with
/// [`InstanceBuilder::reporter`](crate::instance::InstanceBuilder::reporter), runs the journey,
/// and reads the events and the global metadata from the one it kept. It never fails a journey.
///
/// It is available with the `testing` feature, which an application enables under
/// `[dev-dependencies]`.
///
/// # Examples
///
/// ```
/// use itinera::executor::LocalExecutor;
/// use itinera::report::RecordingReporter;
/// use itinera::step::{Outcome, StepDescriptor, step_name};
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Orders;
///
/// let orders = WorkflowDescriptor::builder("orders")
///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
///     .build()?;
/// let recorder = RecordingReporter::default();
/// let instance = orders.instance(Orders).reporter(recorder.clone()).create()?;
/// LocalExecutor::new().run(instance)?;
/// let kinds: Vec<_> = recorder.events().iter().map(|event| event.kind()).collect();
/// assert_eq!(
///     kinds,
///     ["journey_started", "attempt_started", "step_succeeded", "journey_succeeded"]
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, Default)]
pub struct RecordingReporter {
    events: Arc<Mutex<Vec<Event>>>,
    global_metadata: Arc<Mutex<GlobalMetadata>>,
}

impl RecordingReporter {
    /// Every event the reporter and its clones have received so far, in the order they received
    /// them.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::report::RecordingReporter;
    ///
    /// assert!(RecordingReporter::default().events().is_empty());
    /// ```
    #[must_use]
    pub fn events(&self) -> Vec<Event> {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The global metadata the reporter and its clones have received so far, each key with its
    /// latest value, in the order the keys were first set. The journey ID is its first entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::executor::LocalExecutor;
    /// use itinera::report::RecordingReporter;
    /// use itinera::value::AnyValue;
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders").build()?;
    /// let recorder = RecordingReporter::default();
    /// let instance = orders
    ///     .instance(Orders)
    ///     .global_metadata("tenant", "acme".to_string())
    ///     .reporter(recorder.clone())
    ///     .create()?;
    /// LocalExecutor::new().run(instance)?;
    /// let metadata = recorder.global_metadata();
    /// let tenant = metadata.get("tenant").and_then(AnyValue::downcast_ref::<String>);
    /// assert_eq!(tenant.map(String::as_str), Some("acme"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn global_metadata(&self) -> GlobalMetadata {
        self.global_metadata
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Reporter for RecordingReporter {
    fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<(), Error> {
        self.global_metadata
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key, value.clone());
        Ok(())
    }

    fn report(&mut self, event: &Event) -> Result<(), Error> {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::fixtures::started;

    #[test]
    fn a_recording_reporters_clones_share_the_events_and_global_metadata_they_receive() {
        let kept = RecordingReporter::default();
        let mut given = kept.clone();
        given.report(&started()).unwrap();
        given
            .set_global_metadata("tenant", &AnyValue::new("acme".to_string()))
            .unwrap();
        assert_eq!(
            kept.events().iter().map(Event::kind).collect::<Vec<_>>(),
            ["journey_started"]
        );
        assert!(kept.global_metadata().get("tenant").is_some());
    }
}
