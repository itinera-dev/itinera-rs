//! A reporter for tests, which keeps every event it receives.

use std::sync::{Arc, Mutex, PoisonError};

use crate::error::Error;
use crate::event::Event;
use crate::report::Reporter;

/// A reporter for tests: it keeps every event it receives, for the test to read once the journey
/// is over.
///
/// Its clones share what they keep. A test makes one, adds a clone to the workflow instance with
/// [`InstanceBuilder::reporter`](crate::instance::InstanceBuilder::reporter), runs the journey,
/// and reads the events from the one it kept. It never fails a journey.
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
}

impl Reporter for RecordingReporter {
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
    fn a_recording_reporters_clones_share_the_events_they_receive() {
        let kept = RecordingReporter::default();
        let mut given = kept.clone();
        given.report(&started()).unwrap();
        assert_eq!(
            kept.events().iter().map(Event::kind).collect::<Vec<_>>(),
            ["journey_started"]
        );
    }
}
