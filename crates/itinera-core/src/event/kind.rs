//! The kind of an event, without what it carries.

/// The kind of an event, without what it carries: one per event of the catalogue.
///
/// It displays as the specification names the event, in snake_case, for example `step_failed`,
/// and parses from that name.
///
/// # Examples
///
/// ```
/// use itinera::event::{Event, EventKind};
///
/// fn is_last(event: &Event) -> bool {
///     matches!(
///         event.kind(),
///         EventKind::JourneySucceeded | EventKind::JourneyFailed | EventKind::JourneyAborted
///     )
/// }
///
/// assert_eq!(EventKind::StepFailed.to_string(), "step_failed");
/// assert_eq!("step_failed".parse(), Ok(EventKind::StepFailed));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::Display, derive_more::FromStr)]
#[display(rename_all = "snake_case")]
#[from_str(rename_all = "snake_case")]
#[non_exhaustive]
pub enum EventKind {
    /// The kind of [`EventBody::JourneyStarted`](super::EventBody::JourneyStarted).
    JourneyStarted,
    /// The kind of [`EventBody::AttemptStarted`](super::EventBody::AttemptStarted).
    AttemptStarted,
    /// The kind of [`EventBody::InputAdapterSupplied`](super::EventBody::InputAdapterSupplied).
    InputAdapterSupplied,
    /// The kind of [`EventBody::InputAdapterFailed`](super::EventBody::InputAdapterFailed).
    InputAdapterFailed,
    /// The kind of [`EventBody::OptionalInputAbsent`](super::EventBody::OptionalInputAbsent).
    OptionalInputAbsent,
    /// The kind of [`EventBody::ServiceProvided`](super::EventBody::ServiceProvided).
    ServiceProvided,
    /// The kind of [`EventBody::ServiceProviderFailed`](super::EventBody::ServiceProviderFailed).
    ServiceProviderFailed,
    /// The kind of [`EventBody::StepSucceeded`](super::EventBody::StepSucceeded).
    StepSucceeded,
    /// The kind of [`EventBody::StepFailed`](super::EventBody::StepFailed).
    StepFailed,
    /// The kind of [`EventBody::StepSkipped`](super::EventBody::StepSkipped).
    StepSkipped,
    /// The kind of [`EventBody::StepAbnormalTermination`](super::EventBody::StepAbnormalTermination).
    StepAbnormalTermination,
    /// The kind of [`EventBody::HookCalled`](super::EventBody::HookCalled).
    HookCalled,
    /// The kind of [`EventBody::ContributionCommitted`](super::EventBody::ContributionCommitted).
    ContributionCommitted,
    /// The kind of [`EventBody::ContributionsDiscarded`](super::EventBody::ContributionsDiscarded).
    ContributionsDiscarded,
    /// The kind of [`EventBody::DataOverwritten`](super::EventBody::DataOverwritten).
    DataOverwritten,
    /// The kind of [`EventBody::GlobalMetadataSet`](super::EventBody::GlobalMetadataSet).
    GlobalMetadataSet,
    /// The kind of [`EventBody::JourneyAborted`](super::EventBody::JourneyAborted).
    JourneyAborted,
    /// The kind of [`EventBody::StepRetrying`](super::EventBody::StepRetrying).
    StepRetrying,
    /// The kind of [`EventBody::StepGivenUp`](super::EventBody::StepGivenUp).
    StepGivenUp,
    /// The kind of [`EventBody::StepRescuing`](super::EventBody::StepRescuing).
    StepRescuing,
    /// The kind of [`EventBody::StepRescued`](super::EventBody::StepRescued).
    StepRescued,
    /// The kind of [`EventBody::JourneySucceeded`](super::EventBody::JourneySucceeded).
    JourneySucceeded,
    /// The kind of [`EventBody::JourneyFailed`](super::EventBody::JourneyFailed).
    JourneyFailed,
    /// The kind of [`EventBody::StepInfo`](super::EventBody::StepInfo).
    StepInfo,
    /// The kind of [`EventBody::StepWarning`](super::EventBody::StepWarning).
    StepWarning,
    /// The kind of [`EventBody::StepError`](super::EventBody::StepError).
    StepError,
    /// The kind of [`EventBody::JourneyInfo`](super::EventBody::JourneyInfo).
    JourneyInfo,
    /// The kind of [`EventBody::JourneyWarning`](super::EventBody::JourneyWarning).
    JourneyWarning,
    /// The kind of [`EventBody::JourneyError`](super::EventBody::JourneyError).
    JourneyError,
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::journey_started(EventKind::JourneyStarted, "journey_started")]
    #[case::attempt_started(EventKind::AttemptStarted, "attempt_started")]
    #[case::input_adapter_supplied(EventKind::InputAdapterSupplied, "input_adapter_supplied")]
    #[case::input_adapter_failed(EventKind::InputAdapterFailed, "input_adapter_failed")]
    #[case::optional_input_absent(EventKind::OptionalInputAbsent, "optional_input_absent")]
    #[case::service_provided(EventKind::ServiceProvided, "service_provided")]
    #[case::service_provider_failed(EventKind::ServiceProviderFailed, "service_provider_failed")]
    #[case::step_succeeded(EventKind::StepSucceeded, "step_succeeded")]
    #[case::step_failed(EventKind::StepFailed, "step_failed")]
    #[case::step_skipped(EventKind::StepSkipped, "step_skipped")]
    #[case::step_abnormal_termination(
        EventKind::StepAbnormalTermination,
        "step_abnormal_termination"
    )]
    #[case::hook_called(EventKind::HookCalled, "hook_called")]
    #[case::contribution_committed(EventKind::ContributionCommitted, "contribution_committed")]
    #[case::contributions_discarded(EventKind::ContributionsDiscarded, "contributions_discarded")]
    #[case::data_overwritten(EventKind::DataOverwritten, "data_overwritten")]
    #[case::global_metadata_set(EventKind::GlobalMetadataSet, "global_metadata_set")]
    #[case::journey_aborted(EventKind::JourneyAborted, "journey_aborted")]
    #[case::step_retrying(EventKind::StepRetrying, "step_retrying")]
    #[case::step_given_up(EventKind::StepGivenUp, "step_given_up")]
    #[case::step_rescuing(EventKind::StepRescuing, "step_rescuing")]
    #[case::step_rescued(EventKind::StepRescued, "step_rescued")]
    #[case::journey_succeeded(EventKind::JourneySucceeded, "journey_succeeded")]
    #[case::journey_failed(EventKind::JourneyFailed, "journey_failed")]
    #[case::step_info(EventKind::StepInfo, "step_info")]
    #[case::step_warning(EventKind::StepWarning, "step_warning")]
    #[case::step_error(EventKind::StepError, "step_error")]
    #[case::journey_info(EventKind::JourneyInfo, "journey_info")]
    #[case::journey_warning(EventKind::JourneyWarning, "journey_warning")]
    #[case::journey_error(EventKind::JourneyError, "journey_error")]
    fn kinds_display_as_the_specification_writes_them(
        #[case] kind: EventKind,
        #[case] written: &str,
    ) {
        assert_eq!(kind.to_string(), written);
    }

    #[rstest]
    #[case::its_own_name("step_failed", true)]
    #[case::another_casing("Step_Failed", false)]
    #[case::a_name_outside_the_catalogue("step_started", false)]
    fn a_kind_parses_only_from_its_own_name(#[case] name: &str, #[case] parses: bool) {
        assert_eq!(name.parse::<EventKind>().is_ok(), parses);
    }
}
