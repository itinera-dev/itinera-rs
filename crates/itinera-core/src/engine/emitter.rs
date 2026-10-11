//! Emitting the journey's events: making them, delivering them through its dispatcher, and
//! relaying those that steps and hooks emit.

use std::future::Future;
use std::num::NonZeroU64;
use std::pin::Pin;
use std::time::SystemTime;

use super::Journey;
use super::end::Aborted;
use crate::error::{Error, Interrupted};
use crate::event::{Event, EventBody, JourneyAbort, Timestamp};
use crate::journey::{Abort, JourneyId, JourneyStatus};
use crate::report::Dispatcher;
use crate::step::StepName;
use crate::workflow::WorkflowName;

/// Where the emitter reads the time: the system's clock, or a test's.
pub(crate) type Clock = fn() -> SystemTime;

/// Makes the journey's events, numbering them from 1 and stamping them with the time.
pub(crate) struct Emitter {
    journey_id: JourneyId,
    workflow: WorkflowName,
    next: NonZeroU64,
    clock: Clock,
}

impl Emitter {
    pub(crate) fn new(journey_id: JourneyId, workflow: WorkflowName, clock: Clock) -> Self {
        Self {
            journey_id,
            workflow,
            next: NonZeroU64::MIN,
            clock,
        }
    }

    pub(crate) fn next(&mut self, body: EventBody) -> Event {
        let sequence = self.next;
        self.next = sequence.saturating_add(1);
        Event {
            sequence,
            timestamp: Timestamp::from((self.clock)()),
            journey_id: self.journey_id.clone(),
            workflow: self.workflow,
            body,
        }
    }
}

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

/// An event a step or hook emitted, delivered once awaited.
pub(crate) type Emitted<'a> = Pin<Box<dyn Future<Output = Result<(), Interrupted>> + Send + 'a>>;

/// The journey, as the handles of steps and hooks emit through it.
pub(crate) trait Emitting: Send {
    /// Delivers an event a step or hook emitted, unless a reporter failed while it ran.
    fn relay(&mut self, body: EventBody) -> Emitted<'_>;
}

impl<D: Delivery> Journey<D> {
    /// Emits one event, failing with the error of the reporter that failed on it, or else of
    /// the dispatcher.
    pub(super) async fn emit(&mut self, body: EventBody) -> Result<(), Box<Aborted>> {
        let event = self.emitter.next(body);
        let delivered = self.delivery.deliver(&event).await;
        match self.failures.take().map_or(delivered, Err) {
            Ok(()) => Ok(()),
            Err(error) => Err(reporter_failed(self.step, error)),
        }
    }

    /// Emits an event for a step or hook, unless a reporter failed while it ran. A failure now is
    /// recorded, and interrupts it.
    async fn relay_interruptibly(&mut self, body: EventBody) -> Result<(), Interrupted> {
        if self.interrupted.is_some() {
            return Err(Interrupted);
        }
        match self.emit(body).await {
            Ok(()) => Ok(()),
            Err(aborted) => {
                self.interrupted = Some(aborted);
                Err(Interrupted)
            }
        }
    }

    /// Ends the journey as aborted. A failure while `journey_aborted` itself is delivered is
    /// ignored.
    pub(super) async fn abort(&mut self, Aborted { abort, reported }: Aborted) -> JourneyStatus {
        let event = self
            .emitter
            .next(EventBody::JourneyAborted { abort: reported });
        let _ignored = self.delivery.deliver(&event).await;
        JourneyStatus::Aborted(abort)
    }
}

impl<D: Delivery> Emitting for Journey<D> {
    fn relay(&mut self, body: EventBody) -> Emitted<'_> {
        Box::pin(self.relay_interruptibly(body))
    }
}

fn reporter_failed(step: Option<StepName>, error: Error) -> Box<Aborted> {
    let reported = JourneyAbort::ReporterFailed {
        step,
        error: error.to_string(),
    };
    Box::new(Aborted {
        abort: Abort::ReporterFailed(error),
        reported,
    })
}
