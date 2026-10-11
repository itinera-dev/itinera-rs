//! The dispatcher factory the executor is given, and the dispatchers it makes, which hold the
//! runner's recorder and behave as the scenario says.

use itinera::error::Error;
use itinera::event::Event;
use itinera::report::{
    AsyncDispatcher, AsyncDispatcherFactory, BoxedReporter, DefaultDispatcher, Dispatcher,
    DispatcherFactory, Reporter,
};

use super::Recorder;
use crate::model::Holding;

/// Makes the dispatchers the executor is given: each holds the same recorder, before any
/// reporter the executor adds, and behaves as the scenario says.
#[derive(Debug)]
pub(crate) struct RecordingFactory {
    recorder: Recorder,
    behaviour: Behaviour,
}

/// Whether the factory makes dispatchers, and what they do besides delivering to the recorder.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Behaviour {
    Makes(Holding),
    FailsToCreate,
}

impl RecordingFactory {
    pub(crate) fn new(recorder: Recorder, behaviour: Behaviour) -> Self {
        Self {
            recorder,
            behaviour,
        }
    }

    fn holding(&self) -> Result<Holding, Error> {
        match self.behaviour {
            Behaviour::Makes(holding) => Ok(holding),
            Behaviour::FailsToCreate => Err(Error::msg("the scripted dispatcher factory fails")),
        }
    }
}

impl DispatcherFactory for RecordingFactory {
    type Dispatcher = RecordingDispatcher<DefaultDispatcher>;

    fn create(&mut self) -> Result<Self::Dispatcher, Error> {
        let holding = self.holding()?;
        let mut delivery = DefaultDispatcher::new();
        Dispatcher::add(&mut delivery, Box::new(self.recorder.clone()))?;
        Ok(RecordingDispatcher { delivery, holding })
    }
}

impl AsyncDispatcherFactory for RecordingFactory {
    type Dispatcher = RecordingDispatcher<DefaultDispatcher<BoxedReporter>>;

    async fn create(&mut self) -> Result<Self::Dispatcher, Error> {
        let holding = self.holding()?;
        let mut delivery = DefaultDispatcher::<BoxedReporter>::default();
        let recorder = BoxedReporter::from_reporter(self.recorder.clone());
        AsyncDispatcher::add(&mut delivery, recorder).await?;
        Ok(RecordingDispatcher { delivery, holding })
    }
}

/// A dispatcher that delivers as the default one does, to the recorder first and then to the
/// reporters it accepts, unless its script says otherwise.
#[derive(Debug)]
pub(crate) struct RecordingDispatcher<D> {
    delivery: D,
    holding: Holding,
}

impl<D> RecordingDispatcher<D> {
    /// Whether it adds a reporter, or fails, as its script says.
    fn accepts(&self) -> Result<bool, Error> {
        match self.holding {
            Holding::FailsWhenAdding => Err(Error::msg(
                "the scripted dispatcher fails when a reporter is added",
            )),
            Holding::IgnoresAddedReporters => Ok(false),
            Holding::AddsReporters | Holding::FailsDispatching(_) => Ok(true),
        }
    }

    /// Fails instead of delivering an event of the kind its script names.
    fn refuses(&self, event: &Event) -> Result<(), Error> {
        match self.holding {
            Holding::FailsDispatching(kind) if kind.name() == event.kind() => {
                Err(Error::msg(format!(
                    "the scripted dispatcher fails when dispatching {}",
                    kind.name()
                )))
            }
            _ => Ok(()),
        }
    }
}

impl Dispatcher for RecordingDispatcher<DefaultDispatcher> {
    fn add(&mut self, reporter: Box<dyn Reporter>) -> Result<(), Error> {
        if self.accepts()? {
            self.delivery.add(reporter)?;
        }
        Ok(())
    }

    fn dispatch(&mut self, event: &Event) -> Result<(), Error> {
        self.refuses(event)?;
        self.delivery.dispatch(event)
    }
}

impl AsyncDispatcher for RecordingDispatcher<DefaultDispatcher<BoxedReporter>> {
    async fn add(&mut self, reporter: BoxedReporter) -> Result<(), Error> {
        if self.accepts()? {
            self.delivery.add(reporter).await?;
        }
        Ok(())
    }

    async fn dispatch(&mut self, event: &Event) -> Result<(), Error> {
        self.refuses(event)?;
        self.delivery.dispatch(event).await
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;

    use super::*;

    struct Silent;

    impl Reporter for Silent {
        fn report(&mut self, _event: &Event) -> Result<(), Error> {
            Ok(())
        }
    }

    #[test]
    fn a_factory_scripted_to_fail_makes_no_dispatcher() {
        let mut factory = RecordingFactory::new(Recorder::default(), Behaviour::FailsToCreate);
        assert!(DispatcherFactory::create(&mut factory).is_err());
        assert!(block_on(AsyncDispatcherFactory::create(&mut factory)).is_err());
    }

    #[test]
    fn a_dispatcher_scripted_to_fail_when_adding_refuses_every_reporter() {
        let mut factory = RecordingFactory::new(
            Recorder::default(),
            Behaviour::Makes(Holding::FailsWhenAdding),
        );
        let mut dispatcher = DispatcherFactory::create(&mut factory).unwrap();
        assert!(Dispatcher::add(&mut dispatcher, Box::new(Silent)).is_err());
        let mut dispatcher = block_on(AsyncDispatcherFactory::create(&mut factory)).unwrap();
        let reporter = BoxedReporter::from_reporter(Silent);
        assert!(block_on(AsyncDispatcher::add(&mut dispatcher, reporter)).is_err());
    }
}
