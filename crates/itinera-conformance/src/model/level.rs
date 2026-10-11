//! The levels of the events steps and hooks emit.

use super::ModelError;

/// The level of an event a step or hook emits: `step_info` or `journey_info`, and so on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Level {
    Info,
    Warning,
    Error,
}

impl Level {
    /// Reads the level of an event kind made of this prefix and the level, such as `step_info`.
    pub(crate) fn of(prefix: &str, kind: &str) -> Result<Self, ModelError> {
        match kind
            .strip_prefix(prefix)
            .and_then(|kind| kind.strip_prefix('_'))
        {
            Some("info") => Ok(Self::Info),
            Some("warning") => Ok(Self::Warning),
            Some("error") => Ok(Self::Error),
            _ => Err(ModelError::UnknownEmit(kind.to_owned())),
        }
    }

    /// Whether an event of this kind is one a step or hook emits, rather than an engine event.
    pub(crate) fn is_emitted(kind: &str) -> bool {
        Self::of("step", kind).is_ok() || Self::of("journey", kind).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::a_step_level("step_warning", true)]
    #[case::a_journey_level("journey_info", true)]
    #[case::a_step_outcome("step_failed", false)]
    #[case::a_journey_end("journey_aborted", false)]
    fn only_step_and_journey_levels_are_emitted_events(#[case] kind: &str, #[case] emitted: bool) {
        assert_eq!(Level::is_emitted(kind), emitted);
    }

    #[rstest]
    #[case::by_a_step("step", "step_warning", Some(Level::Warning))]
    #[case::by_the_journey("journey", "journey_error", Some(Level::Error))]
    #[case::with_the_prefix_of_another("step", "journey_info", None)]
    #[case::at_a_level_outside_the_catalogue("step", "step_debug", None)]
    fn an_emitted_kind_names_its_level_after_the_prefix_of_whoever_emits_it(
        #[case] emitter: &str,
        #[case] kind: &str,
        #[case] level: Option<Level>,
    ) {
        assert_eq!(Level::of(emitter, kind).ok(), level);
    }
}
