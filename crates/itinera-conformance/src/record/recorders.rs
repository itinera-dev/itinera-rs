//! Every recorder of a scenario, and the one whose events the sentences about the whole stream
//! read.

use std::collections::BTreeMap;

use super::Recorder;
use crate::model::{Dispatching, Model};

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

    /// A new recorder for the scripted reporter of this name, made with its script, which
    /// replaces the one made before, since each workflow instance makes its own reporters.
    pub(crate) fn renewed(&mut self, name: &str, model: &Model) -> Recorder {
        let recorder = Recorder::scripted(model, name);
        self.named.insert(name.to_owned(), recorder.clone());
        recorder
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::model::Holding;

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
}
