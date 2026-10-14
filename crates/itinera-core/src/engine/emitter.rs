//! Emitting the journey's events: making them, delivering them through its dispatcher, and
//! relaying those that steps and hooks emit, with the global metadata they set.

use std::future::Future;
use std::num::NonZeroU64;
use std::time::SystemTime;

use super::Journey;
use super::end::Aborted;
use crate::error::{Error, Interrupted};
use crate::event::{Event, EventBody, JourneyAbort, Source, Timestamp};
use crate::journey::{Abort, JourneyId, JourneyStatus};
use crate::report::Dispatcher;
use crate::step::StepName;
use crate::value::AnyValue;
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

/// Delivers the journey's events and global metadata through its dispatcher, whichever kind it
/// is.
pub(crate) trait Delivery: Send {
    fn deliver(&mut self, event: &Event) -> impl Future<Output = Result<(), Error>> + Send;

    fn set_global_metadata(
        &mut self,
        key: &str,
        value: &AnyValue,
    ) -> impl Future<Output = Result<(), Error>> + Send;
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

    fn set_global_metadata(
        &mut self,
        key: &str,
        value: &AnyValue,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        std::future::ready(self.dispatcher.set_global_metadata(key, value))
    }
}

/// The journey, as the handles of steps and hooks emit through it.
#[async_trait::async_trait]
pub(crate) trait Emitting: Send {
    /// Delivers an event a step or hook emitted, unless a reporter failed while it ran.
    async fn relay(&mut self, body: EventBody) -> Result<(), Interrupted>;

    /// Sets an entry of global metadata for a step or hook, unless a reporter failed while it
    /// ran.
    async fn set_global_metadata(
        &mut self,
        source: Source,
        key: String,
        value: AnyValue,
    ) -> Result<(), Interrupted>;
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
        let emitted = self.emit(body).await;
        self.interrupt_on(emitted)
    }

    /// Records why a step or hook's call is interrupted, when what it did failed the journey.
    pub(super) fn interrupt_on(
        &mut self,
        done: Result<(), Box<Aborted>>,
    ) -> Result<(), Interrupted> {
        let Err(aborted) = done else {
            return Ok(());
        };
        self.interrupted = Some(aborted);
        Err(Interrupted)
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

#[async_trait::async_trait]
impl<D: Delivery> Emitting for Journey<D> {
    async fn relay(&mut self, body: EventBody) -> Result<(), Interrupted> {
        self.relay_interruptibly(body).await
    }

    async fn set_global_metadata(
        &mut self,
        source: Source,
        key: String,
        value: AnyValue,
    ) -> Result<(), Interrupted> {
        self.set_interruptibly(source, key, value).await
    }
}

pub(super) fn reporter_failed(step: Option<StepName>, error: Error) -> Box<Aborted> {
    let reported = JourneyAbort::ReporterFailed {
        step,
        error: error.to_string(),
    };
    Box::new(Aborted {
        abort: Abort::ReporterFailed(error),
        reported,
    })
}

#[cfg(test)]
mod tests;
