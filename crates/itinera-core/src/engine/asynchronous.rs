use std::future::Future;

use super::guard::Guard;
use super::{Delivery, Failures};
use crate::error::Error;
use crate::event::Event;
use crate::report::{AsyncDispatcher, AsyncReporter, BoxedReporter};

/// An asynchronous dispatcher, awaited.
#[derive(derive_more::From)]
pub(crate) struct Awaited<D> {
    dispatcher: D,
}

impl<D: AsyncDispatcher> Delivery for Awaited<D> {
    fn deliver(&mut self, event: &Event) -> impl Future<Output = Result<(), Error>> + Send {
        self.dispatcher.dispatch(event)
    }
}

impl Failures {
    /// Guards a reporter of either kind.
    pub(crate) fn guard_boxed(&self, reporter: BoxedReporter) -> BoxedReporter {
        BoxedReporter::from_async_reporter(Guarded {
            reporter,
            guard: Guard::new(self.clone()),
        })
    }
}

struct Guarded {
    reporter: BoxedReporter,
    guard: Guard,
}

impl AsyncReporter for Guarded {
    async fn report(&mut self, event: &Event) -> Result<(), Error> {
        if !self.guard.admits(event) {
            return Ok(());
        }
        let reported = self.reporter.report(event).await;
        self.guard.record(event, reported)
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;

    use crate::executor::AsyncLocalExecutor;
    use crate::executor::fixtures::{Shop, entries, mixed_instance};
    use crate::journey::{Abort, JourneyStatus};

    #[test]
    fn an_asynchronous_reporter_that_fails_aborts_the_journey_and_only_journey_aborted_reaches_the_others()
     {
        let (instance, log) = mixed_instance(Shop::failing("fragile", "journey_started"));

        let result = block_on(AsyncLocalExecutor::new().run(instance)).unwrap();

        let JourneyStatus::Aborted(Abort::ReporterFailed(error)) = result.status else {
            panic!(
                "the journey was not aborted by a reporter: {:?}",
                result.status
            );
        };
        assert_eq!(error.to_string(), "fragile failed on journey_started");
        assert_eq!(
            entries(&log),
            [
                "audit journey_started",
                "fragile journey_started",
                "audit journey_aborted",
                "metrics journey_aborted",
            ]
        );
    }
}
