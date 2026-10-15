//! Scripted reporters and dispatchers.

use itinera::event::{Event, EventKind as Kind};

use super::ModelError;

/// A kind of event of the catalogue, read from the name the cases use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, derive_more::Display, derive_more::From)]
pub(crate) struct EventKind {
    kind: Kind,
}

impl EventKind {
    pub(crate) fn named(name: &str) -> Result<Self, ModelError> {
        name.parse::<Kind>()
            .map(Self::from)
            .map_err(|_| ModelError::UnknownEvent(name.to_owned()))
    }

    /// Whether the event is of this kind.
    pub(crate) fn is_of(self, event: &Event) -> bool {
        event.kind() == self.kind
    }
}

/// When a scripted reporter fails.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ReporterFailure {
    /// On the first event it receives.
    First,
    /// Whenever it receives an event of this kind, with this message when one is given.
    On(EventKind, Option<String>),
    /// Whenever it receives an entry of global metadata under this key.
    OnGlobalMetadata(String),
}

/// The dispatchers the executor gets, which hold the reporter the runner records with.
#[derive(Debug, Default, PartialEq)]
pub(crate) enum Dispatching {
    /// The scenario does not say: the runner's recording factory, whose dispatchers deliver
    /// to the runner's own reporter and to the workflow's.
    #[default]
    Unstated,
    /// No factory is given, so the executor uses its default dispatcher.
    Default,
    /// A factory whose dispatchers all hold this reporter.
    Holding {
        reporter: String,
        behaviour: Holding,
    },
    /// A factory that fails when asked for a dispatcher.
    FailingFactory,
}

/// What a dispatcher the scenario gives does besides delivering to its reporter.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum Holding {
    /// Adds the workflow's reporters and delivers to them as well.
    #[default]
    AddsReporters,
    IgnoresAddedReporters,
    FailsWhenAdding,
    /// Fails instead of delivering events of this kind.
    FailsDispatching(EventKind),
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::a_kind_of_the_catalogue("step_retrying", Ok(Kind::StepRetrying))]
    #[case::a_kind_outside_it(
        "step_started",
        Err(ModelError::UnknownEvent("step_started".to_owned()))
    )]
    fn only_the_kinds_of_the_catalogue_are_events(
        #[case] name: &str,
        #[case] read: Result<Kind, ModelError>,
    ) {
        assert_eq!(EventKind::named(name).map(|kind| kind.kind), read);
    }
}
