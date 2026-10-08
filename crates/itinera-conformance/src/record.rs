//! The reporters that record what a journey emits, and the dispatchers that hold them.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use itinera::error::Error;
use itinera::event::Event;
use itinera::report::{
    AsyncDispatcher, AsyncDispatcherFactory, BoxedReporter, DefaultDispatcher, Dispatcher,
    DispatcherFactory, Reporter,
};

use crate::model::{Dispatching, Holding, Model, ReporterFailure};

/// A scripted reporter that records every event it receives, then fails as its script says.
///
/// Clones share what they record, so the scenario reads what the reporter that the executor
/// holds received.
#[derive(Clone, Debug, Default)]
pub(crate) struct Recorder {
    shared: Arc<Mutex<Received>>,
}

#[derive(Debug, Default)]
struct Received {
    events: Vec<Event>,
    failure: Option<ReporterFailure>,
    failed: bool,
}

impl Recorder {
    pub(crate) fn failing(failure: Option<ReporterFailure>) -> Self {
        Self {
            shared: Arc::new(Mutex::new(Received {
                failure,
                ..Received::default()
            })),
        }
    }

    /// A recorder that fails as the scenario scripts the reporter of this name.
    fn scripted(model: &Model, name: &str) -> Self {
        Self::failing(model.reporter_failures.get(name).cloned())
    }

    fn received(&self) -> MutexGuard<'_, Received> {
        self.shared.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The events it received, in order, including those it failed on.
    pub(crate) fn events(&self) -> Vec<Event> {
        self.received().events.clone()
    }

    /// Whether it never failed, so that it received every event dispatched to it.
    pub(crate) fn never_failed(&self) -> bool {
        !self.received().failed
    }
}

impl Reporter for Recorder {
    fn report(&mut self, event: &Event) -> Result<(), Error> {
        let mut received = self.received();
        received.events.push(event.clone());
        let fails = match &received.failure {
            None => None,
            Some(ReporterFailure::First) => (received.events.len() == 1).then_some(None),
            Some(ReporterFailure::On(kind, message)) => kind.is_of(event).then(|| message.clone()),
        };
        match fails {
            None => Ok(()),
            Some(message) => {
                received.failed = true;
                Err(Error::msg(message.unwrap_or_else(|| failure(event))))
            }
        }
    }
}

/// What a scripted reporter fails with when its script gives no message.
fn failure(event: &Event) -> String {
    format!("the scripted reporter fails on {}", event.kind())
}

/// Every recorder of a scenario: the runner's own, and the scripted reporters by name.
#[derive(Debug, Default)]
pub(crate) struct Recorders {
    own: Recorder,
    named: BTreeMap<String, Recorder>,
}

/// Why no stream can be read.
#[derive(Debug, PartialEq, thiserror::Error)]
pub(crate) enum NoStream {
    #[error("no reporter \"{0}\" was made")]
    UnknownReporter(String),
    /// The workflow lists no reporter, so nothing receives its events.
    #[error("the workflow lists no reporter to read the events from")]
    NoReporterListed,
    #[error("every reporter the workflow lists failed, so none received the whole event stream")]
    EveryReporterFailed,
}

impl Recorders {
    /// The recorder of the scripted reporter of this name, made with its script on first use.
    pub(crate) fn named(&mut self, name: &str, model: &Model) -> Recorder {
        self.named
            .entry(name.to_owned())
            .or_insert_with(|| Recorder::scripted(model, name))
            .clone()
    }

    /// The runner's own recorder, held by its dispatchers when the scenario names none.
    pub(crate) fn own(&self) -> Recorder {
        self.own.clone()
    }

    pub(crate) fn get(&self, name: &str) -> Result<&Recorder, NoStream> {
        self.named
            .get(name)
            .ok_or_else(|| NoStream::UnknownReporter(name.to_owned()))
    }

    /// The recorder whose events the sentences about the whole stream read: the runner's own,
    /// the reporter the scenario's dispatchers hold, or, with the default dispatcher, the first
    /// reporter the workflow lists that never failed.
    pub(crate) fn stream(&self, model: &Model) -> Result<&Recorder, NoStream> {
        match &model.dispatching {
            Dispatching::Unstated | Dispatching::FailingFactory => Ok(&self.own),
            Dispatching::Holding { reporter, .. } => self.get(reporter),
            Dispatching::Default => {
                let mut listed = model
                    .workflow()
                    .map(|workflow| workflow.reporters.as_slice())
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|name| self.named.get(name))
                    .peekable();
                if listed.peek().is_none() {
                    return Err(NoStream::NoReporterListed);
                }
                listed
                    .find(|recorder| recorder.never_failed())
                    .ok_or(NoStream::EveryReporterFailed)
            }
        }
    }
}

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
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    use super::*;

    fn now<T>(future: impl Future<Output = T>) -> T {
        match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("nothing here waits"),
        }
    }

    struct Silent;

    impl Reporter for Silent {
        fn report(&mut self, _event: &Event) -> Result<(), Error> {
            Ok(())
        }
    }

    fn listing(reporters: &[&str]) -> Model {
        let mut model = Model::default();
        model.dispatching = Dispatching::Default;
        model.declare("orders".to_owned(), Vec::new()).unwrap();
        model.workflow_mut().unwrap().reporters =
            reporters.iter().copied().map(str::to_owned).collect();
        model
    }

    fn fail(recorder: &Recorder) {
        recorder.received().failed = true;
    }

    #[test]
    fn with_the_default_dispatcher_the_stream_is_the_first_listed_reporter_that_never_failed() {
        let model = listing(&["audit", "metrics", "trace"]);
        let mut recorders = Recorders::default();
        let audit = recorders.named("audit", &model);
        let metrics = recorders.named("metrics", &model);
        recorders.named("trace", &model);
        assert!(Arc::ptr_eq(
            &recorders.stream(&model).unwrap().shared,
            &audit.shared
        ));
        fail(&audit);
        assert!(Arc::ptr_eq(
            &recorders.stream(&model).unwrap().shared,
            &metrics.shared
        ));
    }

    #[test]
    fn when_every_listed_reporter_failed_there_is_no_stream_to_read() {
        let model = listing(&["audit", "metrics"]);
        let mut recorders = Recorders::default();
        fail(&recorders.named("audit", &model));
        fail(&recorders.named("metrics", &model));
        assert_eq!(
            recorders.stream(&model).unwrap_err(),
            NoStream::EveryReporterFailed
        );
    }

    #[test]
    fn with_the_default_dispatcher_and_no_listed_reporter_there_is_no_stream_to_read() {
        assert_eq!(
            Recorders::default().stream(&listing(&[])).unwrap_err(),
            NoStream::NoReporterListed
        );
    }

    #[test]
    fn a_dispatcher_the_scenario_names_is_read_through_the_reporter_it_holds() {
        let mut model = Model::default();
        model.dispatching = Dispatching::Holding {
            reporter: "audit".to_owned(),
            behaviour: Holding::AddsReporters,
        };
        let mut recorders = Recorders::default();
        assert_eq!(
            recorders.stream(&model).unwrap_err(),
            NoStream::UnknownReporter("audit".to_owned())
        );
        let audit = recorders.named("audit", &model);
        assert!(Arc::ptr_eq(
            &recorders.stream(&model).unwrap().shared,
            &audit.shared
        ));
        model.dispatching = Dispatching::Unstated;
        assert!(Arc::ptr_eq(
            &recorders.stream(&model).unwrap().shared,
            &recorders.own().shared
        ));
    }

    #[test]
    fn a_factory_scripted_to_fail_makes_no_dispatcher() {
        let mut factory = RecordingFactory::new(Recorder::default(), Behaviour::FailsToCreate);
        assert!(DispatcherFactory::create(&mut factory).is_err());
        assert!(now(AsyncDispatcherFactory::create(&mut factory)).is_err());
    }

    #[test]
    fn a_dispatcher_scripted_to_fail_when_adding_refuses_every_reporter() {
        let mut factory = RecordingFactory::new(
            Recorder::default(),
            Behaviour::Makes(Holding::FailsWhenAdding),
        );
        let mut dispatcher = DispatcherFactory::create(&mut factory).unwrap();
        assert!(Dispatcher::add(&mut dispatcher, Box::new(Silent)).is_err());
        let mut dispatcher = now(AsyncDispatcherFactory::create(&mut factory)).unwrap();
        let reporter = BoxedReporter::from_reporter(Silent);
        assert!(now(AsyncDispatcher::add(&mut dispatcher, reporter)).is_err());
    }
}
