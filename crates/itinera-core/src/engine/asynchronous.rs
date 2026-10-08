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
