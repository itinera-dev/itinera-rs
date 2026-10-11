//! What an event says, one variant per kind of event.

use super::{GiveUpCause, HookSource, JourneyAbort, JourneyFailure, RequestSource, Source};
use crate::policy::{Lifecycle, PolicyName, RetryCause};
use crate::step::{Reason, StepAttempt, StepName};
use crate::value::AnyValue;
use crate::workflow::AdapterName;

/// What an event says: one variant per kind of event.
///
/// # Examples
///
/// ```
/// use itinera::event::EventBody;
///
/// fn is_decision(body: &EventBody) -> bool {
///     matches!(
///         body,
///         EventBody::StepRetrying { .. }
///             | EventBody::StepGivenUp { .. }
///             | EventBody::JourneySucceeded { .. }
///             | EventBody::JourneyFailed { .. }
///     )
/// }
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum EventBody {
    /// The journey started.
    #[non_exhaustive]
    JourneyStarted {
        /// The keys of the initial data, never their values.
        initial_keys: Vec<String>,
    },
    /// An attempt of a step started.
    #[non_exhaustive]
    AttemptStarted {
        /// The step and attempt.
        step: StepAttempt,
    },
    /// An input adapter supplied a value for a step's input.
    #[non_exhaustive]
    InputAdapterSupplied {
        /// The step and attempt.
        step: StepAttempt,
        /// The input's key.
        key: String,
        /// The adapter's name.
        adapter: AdapterName,
    },
    /// An input adapter failed for a step's input.
    #[non_exhaustive]
    InputAdapterFailed {
        /// The step and attempt.
        step: StepAttempt,
        /// The input's key.
        key: String,
        /// The adapter's name.
        adapter: AdapterName,
    },
    /// An optional request had no value.
    #[non_exhaustive]
    OptionalInputAbsent {
        /// The key that was requested.
        key: String,
        /// Who requested it.
        requester: RequestSource,
    },
    /// An attempt reported Success.
    #[non_exhaustive]
    StepSucceeded {
        /// The step and attempt.
        step: StepAttempt,
    },
    /// An attempt reported Failure.
    #[non_exhaustive]
    StepFailed {
        /// The step and attempt.
        step: StepAttempt,
        /// Whether the failure is retriable.
        retriable: bool,
        /// Why the step failed.
        reason: Reason,
    },
    /// An attempt reported Skipped.
    #[non_exhaustive]
    StepSkipped {
        /// The step and attempt.
        step: StepAttempt,
        /// Why the step was skipped, if it said.
        reason: Option<Reason>,
    },
    /// An error escaped a running step.
    #[non_exhaustive]
    StepAbnormalTermination {
        /// The step and attempt.
        step: StepAttempt,
        /// The error's message.
        message: String,
    },
    /// A hook returned without failing.
    #[non_exhaustive]
    HookCalled {
        /// The hook, and the step and attempt that triggered a step hook.
        hook: HookSource,
        /// The lifecycle it returned, if any.
        lifecycle: Option<Lifecycle>,
    },
    /// A contribution reached the data bag.
    #[non_exhaustive]
    ContributionCommitted {
        /// The contribution's key.
        key: String,
        /// Who contributed it.
        source: Source,
    },
    /// A skipped step's contributions were discarded.
    #[non_exhaustive]
    ContributionsDiscarded {
        /// The step and attempt.
        step: StepAttempt,
    },
    /// A committed key replaced an earlier value.
    #[non_exhaustive]
    DataOverwritten {
        /// The key.
        key: String,
        /// Who contributed the new value.
        source: Source,
    },
    /// The journey was aborted. This is always its last event.
    #[non_exhaustive]
    JourneyAborted {
        /// Why it was aborted, and what that reason carries.
        abort: JourneyAbort,
    },
    /// The executor decided to attempt a step again.
    #[non_exhaustive]
    StepRetrying {
        /// The step and the attempt that failed.
        step: StepAttempt,
        /// Why it will be attempted again. The executor's own rule always decides it.
        cause: RetryCause,
    },
    /// The executor decided not to attempt a step again: it ends as failed.
    #[non_exhaustive]
    StepGivenUp {
        /// The step and its last attempt.
        step: StepAttempt,
        /// Why it will not be attempted again, and who decided it.
        cause: GiveUpCause,
    },
    /// The executor decided that the journey succeeds.
    #[non_exhaustive]
    JourneySucceeded {
        /// The policy whose `on step success` returned `FinishWorkflow`, or `None` when no
        /// step was left and the journey succeeded by default.
        decided_by: Option<PolicyName>,
    },
    /// The executor decided that the journey fails.
    #[non_exhaustive]
    JourneyFailed {
        /// The name of the step that failed, or whose hook returned `FailWorkflow`.
        step: StepName,
        /// Why the journey failed, and who decided it.
        failure: JourneyFailure,
    },
    /// A step reported information.
    #[non_exhaustive]
    StepInfo {
        /// The step and attempt.
        step: StepAttempt,
        /// The step's message.
        message: String,
        /// The step's data, if any.
        data: Option<AnyValue>,
    },
    /// A step reported a warning.
    #[non_exhaustive]
    StepWarning {
        /// The step and attempt.
        step: StepAttempt,
        /// The step's message.
        message: String,
        /// The step's data, if any.
        data: Option<AnyValue>,
    },
    /// A step reported an error, which does not change its outcome.
    #[non_exhaustive]
    StepError {
        /// The step and attempt.
        step: StepAttempt,
        /// The step's message.
        message: String,
        /// The step's data, if any.
        data: Option<AnyValue>,
    },
    /// A hook reported information.
    #[non_exhaustive]
    JourneyInfo {
        /// The hook, and the step and attempt that triggered a step hook.
        hook: HookSource,
        /// The hook's message.
        message: String,
        /// The hook's data, if any.
        data: Option<AnyValue>,
    },
    /// A hook reported a warning.
    #[non_exhaustive]
    JourneyWarning {
        /// The hook, and the step and attempt that triggered a step hook.
        hook: HookSource,
        /// The hook's message.
        message: String,
        /// The hook's data, if any.
        data: Option<AnyValue>,
    },
    /// A hook reported an error, which does not change what it returns.
    #[non_exhaustive]
    JourneyError {
        /// The hook, and the step and attempt that triggered a step hook.
        hook: HookSource,
        /// The hook's message.
        message: String,
        /// The hook's data, if any.
        data: Option<AnyValue>,
    },
}
impl EventBody {
    /// The kind of event, in snake_case, for example `step_failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::event::EventBody;
    ///
    /// fn is_emitted_by_a_step(body: &EventBody) -> bool {
    ///     matches!(body.kind(), "step_info" | "step_warning" | "step_error")
    /// }
    /// ```
    pub fn kind(&self) -> &'static str {
        match self {
            Self::JourneyStarted { .. } => "journey_started",
            Self::AttemptStarted { .. } => "attempt_started",
            Self::InputAdapterSupplied { .. } => "input_adapter_supplied",
            Self::InputAdapterFailed { .. } => "input_adapter_failed",
            Self::OptionalInputAbsent { .. } => "optional_input_absent",
            Self::StepSucceeded { .. } => "step_succeeded",
            Self::StepFailed { .. } => "step_failed",
            Self::StepSkipped { .. } => "step_skipped",
            Self::StepAbnormalTermination { .. } => "step_abnormal_termination",
            Self::HookCalled { .. } => "hook_called",
            Self::ContributionCommitted { .. } => "contribution_committed",
            Self::ContributionsDiscarded { .. } => "contributions_discarded",
            Self::DataOverwritten { .. } => "data_overwritten",
            Self::JourneyAborted { .. } => "journey_aborted",
            Self::StepRetrying { .. } => "step_retrying",
            Self::StepGivenUp { .. } => "step_given_up",
            Self::JourneySucceeded { .. } => "journey_succeeded",
            Self::JourneyFailed { .. } => "journey_failed",
            Self::StepInfo { .. } => "step_info",
            Self::StepWarning { .. } => "step_warning",
            Self::StepError { .. } => "step_error",
            Self::JourneyInfo { .. } => "journey_info",
            Self::JourneyWarning { .. } => "journey_warning",
            Self::JourneyError { .. } => "journey_error",
        }
    }
}
