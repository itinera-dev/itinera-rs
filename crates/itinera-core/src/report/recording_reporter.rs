//! A reporter for tests, which keeps every event and the global metadata it receives.

use std::sync::{Arc, Mutex, PoisonError};

use crate::error::Error;
use crate::event::Event;
use crate::report::{GlobalMetadata, Reporter};
use crate::value::AnyValue;

mod path;

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

    /// The path of the journey the reporter and its clones have received so far: one line for
    /// each event of its flow, in the order they received them, so that a test can compare it
    /// with the path it expects, or with an approved file through `path().join("\n")`.
    ///
    /// Only the flow has lines: the outcome of each attempt, each decision on a step, and how the
    /// journey ended. Inputs, services, hooks, contributions, the events that steps, hooks and
    /// service providers emit, and global metadata have none, and no line holds a timestamp, a
    /// sequence number or an ID, so a path changes only when the flow does. The lines read:
    ///
    /// - `charge attempt 1 succeeded`, `charge attempt 1 failed: declined`, where `declined` is
    ///   the reason's code, `charge attempt 1 skipped`, with `: ` and the reason's code when it
    ///   has one, and `charge attempt 1 terminated abnormally: timeout`, with the message. A
    ///   rescue step's attempts read the same under its own name, such as `charge_rescue`.
    /// - `charge retrying`, `charge given up: retries exhausted`, or the other cause, `failure`
    ///   or `abnormal termination`, and `charge given up: FailWorkflow from fraud_check` when a
    ///   hook of the policy `fraud_check` gave it up.
    /// - `charge rescuing with charge_rescue` and `charge rescued by charge_rescue`.
    /// - `journey succeeded`, or `journey succeeded, decided by early_exit` when a hook of the
    ///   policy `early_exit` decided it. `journey failed at charge: declined`, with the code of the
    ///   reason of the step's last attempt or of the one a hook's `FailWorkflow` carried, or
    ///   `journey failed at charge: timeout`, with the error's message when the last attempt ended
    ///   in an abnormal termination. The cause is not repeated, since the line before gives it.
    ///   `journey aborted at charge: step could not be built`, with the abort's reason, or
    ///   `journey aborted: reporter failed` when the abort concerns no step.
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
    /// let charge = StepDescriptor::new(step_name!("charge"), || {
    ///     Ok(Outcome::failure("declined"))
    /// });
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(charge.retry_budget(1))
    ///     .build()?;
    /// let recorder = RecordingReporter::default();
    /// let instance = orders.instance(Orders).reporter(recorder.clone()).create()?;
    /// LocalExecutor::new().run(instance)?;
    /// assert_eq!(
    ///     recorder.path(),
    ///     [
    ///         "charge attempt 1 failed: declined",
    ///         "charge retrying",
    ///         "charge attempt 2 failed: declined",
    ///         "charge given up: retries exhausted",
    ///         "journey failed at charge: declined",
    ///     ]
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn path(&self) -> Vec<String> {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter_map(path::line)
            .collect()
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
    ///     .reporters_global_metadata("tenant", "acme".to_string())
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
    use crate::engine::fixtures::CHARGE;
    use crate::executor::LocalExecutor;
    use crate::report::fixtures::started;
    use crate::step::{Outcome, RescueOutcome, StepDescriptor};
    use crate::workflow::fixtures::{Orders, orders};

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

    #[test]
    fn a_path_lists_the_flow_of_the_journey_in_the_order_it_went() {
        let charge = StepDescriptor::new(CHARGE, || Ok(Outcome::failure("declined")))
            .rescue(|| Ok(RescueOutcome::success()));
        let recorder = RecordingReporter::default();
        let instance = orders()
            .step(charge)
            .build()
            .unwrap()
            .instance(Orders)
            .reporter(recorder.clone())
            .create()
            .unwrap();
        LocalExecutor::new().run(instance).unwrap();
        assert_eq!(
            recorder.path(),
            [
                "charge attempt 1 failed: declined",
                "charge rescuing with charge_rescue",
                "charge_rescue attempt 1 succeeded",
                "charge rescued by charge_rescue",
                "charge attempt 2 failed: declined",
                "charge given up: failure",
                "journey failed at charge: declined",
            ]
        );
    }
}
