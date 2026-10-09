//! The engine both executors share: it runs one journey, emitting its events, and makes its
//! result.

use std::future::Future;

use crate::error::Error;
use crate::event::{Event, EventBody, JourneyAbort};
use crate::instance::WorkflowInstance;
use crate::journey::{Abort, JourneyResult, JourneyStatus};
use crate::mode::Mode;
use crate::report::Dispatcher;
use crate::step::{StepAttempt, StepName};
use crate::workflow::WorkflowDescriptor;

#[cfg(feature = "async")]
mod asynchronous;
mod emitter;
mod guard;

#[cfg(feature = "async")]
pub(crate) use asynchronous::Awaited;
pub(crate) use emitter::Clock;
pub(crate) use guard::Failures;

use emitter::Emitter;

/// Delivers the journey's events through its dispatcher, whichever kind it is.
pub(crate) trait Delivery: Send {
    fn deliver(&mut self, event: &Event) -> impl Future<Output = Result<(), Error>> + Send;
}

/// A synchronous dispatcher, called at once.
#[derive(derive_more::From)]
pub(crate) struct Inline<D> {
    dispatcher: D,
}

impl<D: Dispatcher> Delivery for Inline<D> {
    fn deliver(&mut self, event: &Event) -> impl Future<Output = Result<(), Error>> + Send {
        std::future::ready(self.dispatcher.dispatch(event))
    }
}

/// Runs one journey of the instance, delivering its events through the dispatcher, whose
/// reporters were guarded by `failures`.
pub(crate) async fn run<I: WorkflowInstance>(
    instance: I,
    delivery: impl Delivery,
    failures: Failures,
    clock: Clock,
) -> JourneyResult {
    let journey_id = instance.journey_id().clone();
    let descriptor = instance.descriptor().clone();
    let initial_keys = instance.data_bag().keys().map(str::to_owned).collect();
    let emitter = Emitter::new(journey_id.clone(), descriptor.name(), clock);
    let mut journey = Journey {
        emitter,
        delivery,
        failures,
        step: None,
    };
    let status = match journey.succeed(&descriptor, initial_keys).await {
        Ok(()) => JourneyStatus::Succeeded {
            data: instance.into_data_bag(),
        },
        Err(error) => journey.abort(error).await,
    };
    JourneyResult { journey_id, status }
}

/// One journey while it runs.
struct Journey<D> {
    emitter: Emitter,
    delivery: D,
    failures: Failures,
    /// The step being run, if any.
    step: Option<StepName>,
}

impl<D: Delivery> Journey<D> {
    /// Runs the journey to its success, or stops at the first reporter or dispatcher that fails.
    ///
    /// It holds no reference to the instance while it awaits, so that the journey's future is
    /// `Send` whenever the instance is.
    async fn succeed<W: Send + Sync + 'static, M: Mode>(
        &mut self,
        descriptor: &WorkflowDescriptor<W, M>,
        initial_keys: Vec<String>,
    ) -> Result<(), Error> {
        self.emit(EventBody::JourneyStarted { initial_keys })
            .await?;
        if let Some(step) = descriptor.step() {
            self.step = Some(step.name());
            let attempt = StepAttempt::first(step.name());
            self.emit(EventBody::AttemptStarted {
                step: attempt.clone(),
            })
            .await?;
            step.run();
            self.emit(EventBody::StepSucceeded { step: attempt })
                .await?;
            self.step = None;
        }
        self.emit(EventBody::JourneySucceeded { decided_by: None })
            .await
    }

    /// Emits one event, failing with the error of the reporter that failed on it, or else of
    /// the dispatcher.
    async fn emit(&mut self, body: EventBody) -> Result<(), Error> {
        let event = self.emitter.next(body);
        let delivered = self.delivery.deliver(&event).await;
        match self.failures.take() {
            Some(error) => Err(error),
            None => delivered,
        }
    }

    /// Ends the journey as aborted by the error of a reporter or dispatcher. A failure while
    /// `journey_aborted` itself is delivered is ignored.
    async fn abort(&mut self, error: Error) -> JourneyStatus {
        let abort = JourneyAbort::ReporterFailed {
            step: self.step.take(),
            error: error.to_string(),
        };
        let event = self.emitter.next(EventBody::JourneyAborted { abort });
        let _ignored = self.delivery.deliver(&event).await;
        JourneyStatus::Aborted(Abort::ReporterFailed(error))
    }
}

/// Runs a future that never waits to its end, without a runtime.
///
/// A synchronous workflow has nothing to wait for, so its journey is ready the first time it is
/// polled.
pub(crate) fn finish<T>(future: impl Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return value;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::event::{Event, Timestamp};
    use crate::report::{DefaultDispatcher, Reporter};
    use crate::workflow::{WorkflowDescriptor, WorkflowName};

    struct Orders;

    #[derive(Default)]
    struct Recording {
        events: Arc<Mutex<Vec<Event>>>,
    }

    impl Reporter for Recording {
        fn report(&mut self, event: &Event) -> Result<(), Error> {
            self.events.lock().unwrap().push(event.clone());
            Ok(())
        }
    }

    fn noon() -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(1_700_000_000)
    }

    fn sequence(event: &Event) -> NonZeroU64 {
        event.sequence
    }

    #[test]
    fn events_are_numbered_from_one_and_carry_the_journey_the_workflow_and_the_time() {
        let recording = Recording::default();
        let events = Arc::clone(&recording.events);
        let failures = Failures::default();
        let mut dispatcher = DefaultDispatcher::new();
        dispatcher.add(failures.guard(Box::new(recording))).unwrap();
        let workflow = WorkflowDescriptor::builder("orders")
            .step(StepName::new("charge"), || {})
            .id_generator(|_: &Orders, _| Ok("order-7".to_string()))
            .build();
        let instance = workflow
            .instance(Orders)
            .data("amount", 42_i64)
            .create()
            .unwrap();

        finish(run(instance, Inline::from(dispatcher), failures, noon));

        let events = events.lock().unwrap();
        let sequences: Vec<u64> = events.iter().map(sequence).map(NonZeroU64::get).collect();
        assert_eq!(sequences, [1, 2, 3, 4]);
        for event in events.iter() {
            assert_eq!(event.journey_id.to_string(), "order-7");
            assert_eq!(event.workflow, WorkflowName::from("orders"));
            assert_eq!(event.timestamp, Timestamp::from(noon()));
        }
        let Some(EventBody::JourneyStarted { initial_keys }) = events.first().map(|e| &e.body)
        else {
            panic!("the first event is not journey_started");
        };
        assert_eq!(initial_keys, &["amount"]);
    }
}
