//! Scripted reporters and dispatchers.

use super::ModelError;

/// The kinds of event of the catalogue, by the names the cases use.
const EVENT_KINDS: [&str; 24] = [
    "journey_started",
    "attempt_started",
    "input_adapter_supplied",
    "input_adapter_failed",
    "optional_input_absent",
    "step_succeeded",
    "step_failed",
    "step_skipped",
    "step_abnormal_termination",
    "hook_called",
    "contribution_committed",
    "contributions_discarded",
    "data_overwritten",
    "journey_aborted",
    "step_retrying",
    "step_given_up",
    "journey_succeeded",
    "journey_failed",
    "step_info",
    "step_warning",
    "step_error",
    "journey_info",
    "journey_warning",
    "journey_error",
];

/// The name of a kind of event of the catalogue, as `Event::kind` returns it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EventKind(&'static str);

impl EventKind {
    pub(crate) fn named(name: &str) -> Result<Self, ModelError> {
        for kind in EVENT_KINDS {
            if kind == name {
                return Ok(Self(kind));
            }
        }
        Err(ModelError::UnknownEvent(name.to_owned()))
    }

    pub(crate) fn name(self) -> &'static str {
        self.0
    }
}

/// When a scripted reporter fails.
#[derive(Debug, PartialEq)]
pub(crate) enum ReporterFailure {
    /// On the first event it receives.
    First,
    /// Whenever it receives an event of this kind, with this message when one is given.
    On(EventKind, Option<String>),
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
#[derive(Debug, Default, PartialEq)]
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
    use super::*;

    #[test]
    fn only_the_kinds_of_the_catalogue_are_events() {
        assert_eq!(
            EventKind::named("step_retrying").unwrap().name(),
            "step_retrying"
        );
        assert_eq!(
            EventKind::named("step_started").unwrap_err(),
            ModelError::UnknownEvent("step_started".to_owned())
        );
    }
}
