use std::fmt;
use std::num::{NonZeroU32, NonZeroU64};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Serialize, Serializer};

use crate::value::{AnyValue, Value};

/// The identifier of one journey, unique to it.
///
/// # Examples
///
/// ```
/// use itinera::JourneyId;
///
/// let id = JourneyId::new("order-42");
/// assert_eq!(id.as_str(), "order-42");
/// assert_eq!(id.to_string(), "order-42");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct JourneyId(String);

impl JourneyId {
    /// Makes a journey ID from its text.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::JourneyId;
    ///
    /// assert_eq!(JourneyId::new(String::from("a")), JourneyId::new("a"));
    /// ```
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The journey ID as text.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::JourneyId;
    ///
    /// assert_eq!(JourneyId::new("order-42").as_str(), "order-42");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for JourneyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A step and one of its attempts, counted from 1.
///
/// # Examples
///
/// ```
/// use itinera::StepAttempt;
///
/// fn describe(attempt: &StepAttempt) -> String {
///     format!("{}, attempt {}", attempt.step, attempt.attempt)
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct StepAttempt {
    /// The step's name.
    pub step: String,
    /// The attempt number, from 1.
    pub attempt: NonZeroU32,
}

/// The name of a hook.
///
/// It displays as the specification writes it, for example `on step success`.
///
/// # Examples
///
/// ```
/// use itinera::HookName;
///
/// assert_eq!(HookName::OnStepRetry.to_string(), "on step retry");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub enum HookName {
    /// Called after an attempt that succeeded.
    #[serde(rename = "on step success")]
    OnStepSuccess,
    /// Called after a step was given up.
    #[serde(rename = "on step failure")]
    OnStepFailure,
    /// Called before a step is attempted again.
    #[serde(rename = "on step retry")]
    OnStepRetry,
    /// Called after an attempt ended in an abnormal termination.
    #[serde(rename = "on step abnormal termination")]
    OnStepAbnormalTermination,
    /// Called once the journey has succeeded.
    #[serde(rename = "on workflow success")]
    OnWorkflowSuccess,
    /// Called once the journey has failed.
    #[serde(rename = "on workflow failure")]
    OnWorkflowFailure,
}

impl HookName {
    /// The hook's name, as the specification writes it.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::HookName;
    ///
    /// assert_eq!(HookName::OnWorkflowFailure.as_str(), "on workflow failure");
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OnStepSuccess => "on step success",
            Self::OnStepFailure => "on step failure",
            Self::OnStepRetry => "on step retry",
            Self::OnStepAbnormalTermination => "on step abnormal termination",
            Self::OnWorkflowSuccess => "on workflow success",
            Self::OnWorkflowFailure => "on workflow failure",
        }
    }
}

impl fmt::Display for HookName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A hook of a named policy.
///
/// # Examples
///
/// ```
/// use itinera::HookRef;
///
/// fn describe(hook: &HookRef) -> String {
///     format!("{}, {}", hook.policy, hook.hook)
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct HookRef {
    /// The policy's name.
    pub policy: String,
    /// The hook.
    pub hook: HookName,
}

/// A hook that was called, and, for a step hook, the step and attempt that triggered it.
///
/// # Examples
///
/// ```
/// use itinera::HookSource;
///
/// fn triggered_by_a_step(source: &HookSource) -> bool {
///     source.step.is_some()
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct HookSource {
    /// The policy's name.
    pub policy: String,
    /// The hook.
    pub hook: HookName,
    /// The step and attempt that triggered a step hook; `None` for a workflow hook.
    #[serde(flatten)]
    pub step: Option<StepAttempt>,
}

/// Why a step failed or was skipped, or why a hook failed the journey: a code, an optional
/// message and optional details.
///
/// The details are a [`Value`], captured when the reason is made.
///
/// # Examples
///
/// ```
/// use itinera::Reason;
///
/// let reason = Reason::new("card-declined")
///     .with_message("the card was declined")
///     .with_details(3_i64);
/// assert_eq!(reason.code(), "card-declined");
/// assert_eq!(reason.message(), Some("the card was declined"));
/// assert_eq!(reason.details().and_then(|d| d.downcast_ref::<i64>()), Some(&3));
/// ```
#[derive(Clone, Debug, Serialize)]
pub struct Reason {
    code: String,
    message: Option<String>,
    details: Option<AnyValue>,
}

impl Reason {
    /// Makes a reason with a code, and no message or details.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::Reason;
    ///
    /// let reason = Reason::new("out-of-stock");
    /// assert_eq!(reason.code(), "out-of-stock");
    /// assert!(reason.message().is_none());
    /// assert!(reason.details().is_none());
    /// ```
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: None,
            details: None,
        }
    }

    /// Gives the reason a message.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::Reason;
    ///
    /// let reason = Reason::new("out-of-stock").with_message("none left");
    /// assert_eq!(reason.message(), Some("none left"));
    /// ```
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Gives the reason details, which must be a [`Value`].
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::Reason;
    ///
    /// let reason = Reason::new("out-of-stock").with_details(vec!["SKU-1".to_string()]);
    /// assert!(reason.details().is_some());
    /// ```
    pub fn with_details<T: Value>(mut self, details: T) -> Self {
        self.details = Some(AnyValue::new(details));
        self
    }

    /// The reason's code.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::Reason;
    ///
    /// assert_eq!(Reason::new("late").code(), "late");
    /// ```
    pub fn code(&self) -> &str {
        &self.code
    }

    /// The reason's message, if it has one.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::Reason;
    ///
    /// assert_eq!(Reason::new("late").message(), None);
    /// ```
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// The reason's details, if it has any.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::Reason;
    ///
    /// let reason = Reason::new("late").with_details(true);
    /// assert_eq!(reason.details().and_then(|d| d.downcast_ref::<bool>()), Some(&true));
    /// ```
    pub fn details(&self) -> Option<&AnyValue> {
        self.details.as_ref()
    }
}

/// A lifecycle a hook returned, which decides what happens next.
///
/// # Examples
///
/// ```
/// use itinera::Lifecycle;
///
/// fn ends_the_journey(lifecycle: &Lifecycle) -> bool {
///     matches!(lifecycle, Lifecycle::FinishWorkflow | Lifecycle::FailWorkflow(_))
/// }
/// ```
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub enum Lifecycle {
    /// The journey succeeds at once.
    FinishWorkflow,
    /// The journey fails at once, with this reason.
    FailWorkflow(Reason),
}

/// Who took a decision: the executor's own rule, or the hook whose returned lifecycle decided it.
///
/// It serializes as `"default"`, or as the policy and hook.
///
/// # Examples
///
/// ```
/// use itinera::DecidedBy;
///
/// fn by_default(decided_by: &DecidedBy) -> bool {
///     matches!(decided_by, DecidedBy::Default)
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DecidedBy {
    /// The executor's own rule.
    Default,
    /// The hook whose returned lifecycle decided it.
    Hook(HookRef),
}

impl Serialize for DecidedBy {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Default => serializer.serialize_str("default"),
            Self::Hook(hook) => hook.serialize(serializer),
        }
    }
}

/// Why a step will be attempted again.
///
/// # Examples
///
/// ```
/// use itinera::RetryCause;
///
/// assert_eq!(RetryCause::RetriableFailure.to_string(), "retriable failure");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub enum RetryCause {
    /// The attempt reported a retriable failure.
    #[serde(rename = "retriable failure")]
    RetriableFailure,
    /// The attempt ended in an abnormal termination, and the step allows retrying it.
    #[serde(rename = "abnormal termination")]
    AbnormalTermination,
}

impl fmt::Display for RetryCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::RetriableFailure => "retriable failure",
            Self::AbnormalTermination => "abnormal termination",
        })
    }
}

/// Why a step will not be attempted again.
///
/// # Examples
///
/// ```
/// use itinera::GiveUpCause;
///
/// assert_eq!(GiveUpCause::RetriesExhausted.to_string(), "retries exhausted");
/// ```
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub enum GiveUpCause {
    /// The attempt reported a failure that is not retriable.
    #[serde(rename = "failure")]
    Failure,
    /// The attempt ended in an abnormal termination, and the step does not allow retrying it.
    #[serde(rename = "abnormal termination")]
    AbnormalTermination,
    /// The attempt reported a retriable failure, and the retry budget is spent.
    #[serde(rename = "retries exhausted")]
    RetriesExhausted,
    /// A hook returned `FailWorkflow`, with this reason.
    FailWorkflow(Reason),
}

impl fmt::Display for GiveUpCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Failure => "failure",
            Self::AbnormalTermination => "abnormal termination",
            Self::RetriesExhausted => "retries exhausted",
            Self::FailWorkflow(_) => "FailWorkflow",
        })
    }
}

/// Why a journey failed.
///
/// # Examples
///
/// ```
/// use itinera::Failure;
///
/// fn message(failure: &Failure) -> String {
///     match failure {
///         Failure::Failed(reason)
///         | Failure::RetriesExhausted(reason)
///         | Failure::FailedByPolicy(reason) => reason.code().to_string(),
///         Failure::AbnormalTermination(message) => message.clone(),
///         _ => String::new(),
///     }
/// }
/// ```
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Failure {
    /// A step reported a failure that is not retriable, with this reason.
    Failed(Reason),
    /// A step's retry budget was spent; this is the reason of its last failure.
    RetriesExhausted(Reason),
    /// A step ended in an abnormal termination, with this error message.
    AbnormalTermination(String),
    /// A hook returned `FailWorkflow`, with this reason.
    FailedByPolicy(Reason),
}

/// Why a journey was aborted.
///
/// It displays as the specification writes it. The reasons `invalid lifecycle` and `not a value`
/// cannot happen in Rust, so they are absent.
///
/// # Examples
///
/// ```
/// use itinera::AbortReason;
///
/// assert_eq!(AbortReason::ReporterFailed.to_string(), "reporter failed");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub enum AbortReason {
    /// A step's constructor, or the input adapter resolving one of its inputs, failed.
    #[serde(rename = "step could not be built")]
    StepCouldNotBeBuilt,
    /// A step policy failed while being built for an attempt.
    #[serde(rename = "policy could not be built")]
    PolicyCouldNotBeBuilt,
    /// A required request has no value.
    #[serde(rename = "required data missing")]
    RequiredDataMissing,
    /// A requested value has the wrong type.
    #[serde(rename = "wrong type")]
    WrongType,
    /// A hook, or a role operation it called, failed.
    #[serde(rename = "hook failed")]
    HookFailed,
    /// A reporter, or a dispatcher while dispatching, failed.
    #[serde(rename = "reporter failed")]
    ReporterFailed,
}

impl AbortReason {
    /// The abort reason, as the specification writes it.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::AbortReason;
    ///
    /// assert_eq!(AbortReason::WrongType.as_str(), "wrong type");
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StepCouldNotBeBuilt => "step could not be built",
            Self::PolicyCouldNotBeBuilt => "policy could not be built",
            Self::RequiredDataMissing => "required data missing",
            Self::WrongType => "wrong type",
            Self::HookFailed => "hook failed",
            Self::ReporterFailed => "reporter failed",
        }
    }
}

impl fmt::Display for AbortReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Who requested data: a step for one of its inputs, a hook, or an input adapter.
///
/// # Examples
///
/// ```
/// use itinera::Requester;
///
/// fn adapter(requester: &Requester) -> Option<&str> {
///     match requester {
///         Requester::Adapter(name) => Some(name),
///         _ => None,
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Requester {
    /// The step, for one of its inputs.
    Step,
    /// A hook.
    Hook(HookRef),
    /// The input adapter with this name.
    Adapter(String),
}

/// What an abort concerns, beyond its reason and step.
///
/// # Examples
///
/// ```
/// use itinera::AbortDetails;
///
/// fn key(details: &AbortDetails) -> Option<&str> {
///     match details {
///         AbortDetails::Data { key, .. } => Some(key),
///         _ => None,
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AbortDetails {
    /// The journey was aborted while data was resolved.
    #[non_exhaustive]
    Data {
        /// The key that was requested.
        key: String,
        /// Who requested it.
        requester: Requester,
    },
    /// A step policy could not be built.
    #[non_exhaustive]
    Policy {
        /// The policy's name.
        policy: String,
    },
}

/// Who made a contribution: a step, or a hook.
///
/// # Examples
///
/// ```
/// use itinera::Source;
///
/// fn from_a_hook(source: &Source) -> bool {
///     matches!(source, Source::Hook(_))
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Source {
    /// A step, during this attempt.
    Step(StepAttempt),
    /// A hook.
    Hook(HookSource),
}

/// One event of a journey's event stream.
///
/// Events are made only by itinera's executors. Every event carries a sequence number,
/// increasing from 1 within the journey, a timestamp, the journey ID and the workflow name;
/// what else it carries depends on its [`body`](Event::body).
///
/// It serializes as one map: `kind`, `sequence`, `timestamp` in ISO 8601 in UTC, `journey_id`,
/// `workflow`, and the fields of its body.
///
/// # Examples
///
/// ```
/// use itinera::{Event, EventBody};
///
/// fn summary(event: &Event) -> String {
///     match &event.body {
///         EventBody::StepFailed { step, reason, .. } => {
///             format!("{} failed: {}", step.step, reason.code())
///         }
///         _ => format!("#{} {}", event.sequence, event.kind()),
///     }
/// }
/// ```
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct Event {
    /// The event's position in the journey's event stream, from 1.
    pub sequence: NonZeroU64,
    /// When the event was emitted.
    #[serde(serialize_with = "iso_8601")]
    pub timestamp: SystemTime,
    /// The journey's ID.
    pub journey_id: JourneyId,
    /// The workflow's name.
    pub workflow: String,
    /// What the event says.
    #[serde(flatten)]
    pub body: EventBody,
}

impl Event {
    /// The event's kind, in snake_case, for example `step_failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::Event;
    ///
    /// fn is_last(event: &Event) -> bool {
    ///     matches!(event.kind(), "journey_succeeded" | "journey_failed" | "journey_aborted")
    /// }
    /// ```
    pub fn kind(&self) -> &'static str {
        self.body.kind()
    }
}

/// What an event says: one variant per kind of event.
///
/// # Examples
///
/// ```
/// use itinera::EventBody;
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
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
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
        #[serde(flatten)]
        step: StepAttempt,
    },
    /// An input adapter supplied a value for a step's input.
    #[non_exhaustive]
    InputAdapterSupplied {
        /// The step and attempt.
        #[serde(flatten)]
        step: StepAttempt,
        /// The input's key.
        key: String,
        /// The adapter's name.
        adapter: String,
    },
    /// An input adapter failed for a step's input.
    #[non_exhaustive]
    InputAdapterFailed {
        /// The step and attempt.
        #[serde(flatten)]
        step: StepAttempt,
        /// The input's key.
        key: String,
        /// The adapter's name.
        adapter: String,
    },
    /// An optional request had no value.
    #[non_exhaustive]
    OptionalInputAbsent {
        /// The step and attempt, unless a workflow hook made the request.
        #[serde(flatten)]
        step: Option<StepAttempt>,
        /// The key that was requested.
        key: String,
        /// Who requested it.
        requester: Requester,
    },
    /// An attempt reported Success.
    #[non_exhaustive]
    StepSucceeded {
        /// The step and attempt.
        #[serde(flatten)]
        step: StepAttempt,
    },
    /// An attempt reported Failure.
    #[non_exhaustive]
    StepFailed {
        /// The step and attempt.
        #[serde(flatten)]
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
        #[serde(flatten)]
        step: StepAttempt,
        /// Why the step was skipped, if it said.
        reason: Option<Reason>,
    },
    /// An error escaped a running step.
    #[non_exhaustive]
    StepAbnormalTermination {
        /// The step and attempt.
        #[serde(flatten)]
        step: StepAttempt,
        /// The error's message.
        message: String,
    },
    /// A hook returned without failing.
    #[non_exhaustive]
    HookCalled {
        /// The hook, and the step and attempt that triggered it.
        #[serde(flatten)]
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
        #[serde(flatten)]
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
        /// The step during which it happened, if any.
        #[serde(flatten)]
        step: Option<StepAttempt>,
        /// Why it was aborted.
        reason: AbortReason,
        /// What it concerns, where the reason needs more.
        details: Option<AbortDetails>,
    },
    /// The executor decided to attempt a step again.
    #[non_exhaustive]
    StepRetrying {
        /// The step and the attempt that failed.
        #[serde(flatten)]
        step: StepAttempt,
        /// Why it will be attempted again.
        cause: RetryCause,
        /// Who decided it.
        decided_by: DecidedBy,
    },
    /// The executor decided not to attempt a step again: it ends as failed.
    #[non_exhaustive]
    StepGivenUp {
        /// The step and its last attempt.
        #[serde(flatten)]
        step: StepAttempt,
        /// Why it will not be attempted again.
        cause: GiveUpCause,
        /// Who decided it.
        decided_by: DecidedBy,
    },
    /// The executor decided that the journey succeeds.
    #[non_exhaustive]
    JourneySucceeded {
        /// Who decided it.
        decided_by: DecidedBy,
    },
    /// The executor decided that the journey fails.
    #[non_exhaustive]
    JourneyFailed {
        /// The step that failed, or whose hook returned `FailWorkflow`, and its last attempt.
        #[serde(flatten)]
        step: StepAttempt,
        /// Why the journey failed.
        failure: Failure,
        /// Who decided it.
        decided_by: DecidedBy,
    },
    /// A step reported information.
    #[non_exhaustive]
    StepInfo {
        /// The step and attempt.
        #[serde(flatten)]
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
        #[serde(flatten)]
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
        #[serde(flatten)]
        step: StepAttempt,
        /// The step's message.
        message: String,
        /// The step's data, if any.
        data: Option<AnyValue>,
    },
    /// A hook reported information.
    #[non_exhaustive]
    JourneyInfo {
        /// The hook, and the step and attempt that triggered it.
        #[serde(flatten)]
        hook: HookSource,
        /// The hook's message.
        message: String,
        /// The hook's data, if any.
        data: Option<AnyValue>,
    },
    /// A hook reported a warning.
    #[non_exhaustive]
    JourneyWarning {
        /// The hook, and the step and attempt that triggered it.
        #[serde(flatten)]
        hook: HookSource,
        /// The hook's message.
        message: String,
        /// The hook's data, if any.
        data: Option<AnyValue>,
    },
    /// A hook reported an error, which does not change what it returns.
    #[non_exhaustive]
    JourneyError {
        /// The hook, and the step and attempt that triggered it.
        #[serde(flatten)]
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
    /// use itinera::EventBody;
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

fn iso_8601<S: Serializer>(timestamp: &SystemTime, serializer: S) -> Result<S::Ok, S::Error> {
    let formatted = rfc_3339(*timestamp)
        .ok_or_else(|| serde::ser::Error::custom("the timestamp is outside the years 0 to 9999"))?;
    serializer.serialize_str(&formatted)
}

const SECONDS_PER_DAY: i64 = 86_400;
const NANOS_PER_SECOND: u32 = 1_000_000_000;

fn rfc_3339(timestamp: SystemTime) -> Option<String> {
    let (seconds, nanos) = seconds_since_epoch(timestamp)?;
    utc(seconds, nanos)
}

fn utc(seconds: i64, nanos: u32) -> Option<String> {
    let (year, month, day) = civil_date(seconds.div_euclid(SECONDS_PER_DAY));
    if !(0..=9999).contains(&year) {
        return None;
    }
    let of_day = seconds.rem_euclid(SECONDS_PER_DAY);
    let mut formatted = format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}",
        of_day / 3600,
        of_day % 3600 / 60,
        of_day % 60
    );
    if nanos > 0 {
        let fraction = format!("{nanos:09}");
        formatted.push('.');
        formatted.push_str(fraction.trim_end_matches('0'));
    }
    formatted.push('Z');
    Some(formatted)
}

fn seconds_since_epoch(timestamp: SystemTime) -> Option<(i64, u32)> {
    match timestamp.duration_since(UNIX_EPOCH) {
        Ok(after) => Some((i64::try_from(after.as_secs()).ok()?, after.subsec_nanos())),
        Err(before) => {
            let before = before.duration();
            let seconds = -i64::try_from(before.as_secs()).ok()?;
            match before.subsec_nanos() {
                0 => Some((seconds, 0)),
                nanos => Some((seconds.checked_sub(1)?, NANOS_PER_SECOND - nanos)),
            }
        }
    }
}

// Howard Hinnant's days-to-civil algorithm, for the proleptic Gregorian calendar.
fn civil_date(days_since_epoch: i64) -> (i64, i64, i64) {
    let days = days_since_epoch + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::time::Duration;

    use serde_json::{Value as Json, json};

    use super::*;

    pub(crate) fn event(sequence: u64, body: EventBody) -> Event {
        Event {
            sequence: NonZeroU64::new(sequence).unwrap(),
            timestamp: UNIX_EPOCH + Duration::from_millis(1_700_000_000_123),
            journey_id: JourneyId::new("order-42"),
            workflow: "orders".to_string(),
            body,
        }
    }

    fn charge(attempt: u32) -> StepAttempt {
        StepAttempt {
            step: "charge".to_string(),
            attempt: NonZeroU32::new(attempt).unwrap(),
        }
    }

    fn audit(hook: HookName, step: Option<StepAttempt>) -> HookSource {
        HookSource {
            policy: "audit".to_string(),
            hook,
            step,
        }
    }

    fn every_kind() -> Vec<EventBody> {
        let reason = || Reason::new("declined");
        vec![
            EventBody::JourneyStarted {
                initial_keys: vec!["amount".to_string()],
            },
            EventBody::AttemptStarted { step: charge(1) },
            EventBody::InputAdapterSupplied {
                step: charge(1),
                key: "amount".to_string(),
                adapter: "pricing".to_string(),
            },
            EventBody::InputAdapterFailed {
                step: charge(1),
                key: "amount".to_string(),
                adapter: "pricing".to_string(),
            },
            EventBody::OptionalInputAbsent {
                step: Some(charge(1)),
                key: "discount".to_string(),
                requester: Requester::Step,
            },
            EventBody::StepSucceeded { step: charge(1) },
            EventBody::StepFailed {
                step: charge(1),
                retriable: true,
                reason: reason(),
            },
            EventBody::StepSkipped {
                step: charge(1),
                reason: None,
            },
            EventBody::StepAbnormalTermination {
                step: charge(1),
                message: "boom".to_string(),
            },
            EventBody::HookCalled {
                hook: audit(HookName::OnStepSuccess, Some(charge(1))),
                lifecycle: None,
            },
            EventBody::ContributionCommitted {
                key: "receipt".to_string(),
                source: Source::Step(charge(1)),
            },
            EventBody::ContributionsDiscarded { step: charge(1) },
            EventBody::DataOverwritten {
                key: "receipt".to_string(),
                source: Source::Step(charge(1)),
            },
            EventBody::JourneyAborted {
                step: None,
                reason: AbortReason::ReporterFailed,
                details: None,
            },
            EventBody::StepRetrying {
                step: charge(1),
                cause: RetryCause::RetriableFailure,
                decided_by: DecidedBy::Default,
            },
            EventBody::StepGivenUp {
                step: charge(2),
                cause: GiveUpCause::RetriesExhausted,
                decided_by: DecidedBy::Default,
            },
            EventBody::JourneySucceeded {
                decided_by: DecidedBy::Default,
            },
            EventBody::JourneyFailed {
                step: charge(2),
                failure: Failure::RetriesExhausted(reason()),
                decided_by: DecidedBy::Default,
            },
            EventBody::StepInfo {
                step: charge(1),
                message: "charging".to_string(),
                data: None,
            },
            EventBody::StepWarning {
                step: charge(1),
                message: "slow".to_string(),
                data: None,
            },
            EventBody::StepError {
                step: charge(1),
                message: "odd".to_string(),
                data: None,
            },
            EventBody::JourneyInfo {
                hook: audit(HookName::OnWorkflowSuccess, None),
                message: "done".to_string(),
                data: None,
            },
            EventBody::JourneyWarning {
                hook: audit(HookName::OnWorkflowSuccess, None),
                message: "late".to_string(),
                data: None,
            },
            EventBody::JourneyError {
                hook: audit(HookName::OnWorkflowFailure, None),
                message: "lost".to_string(),
                data: None,
            },
        ]
    }

    fn to_json(event: &Event) -> Json {
        serde_json::to_value(event).unwrap()
    }

    #[test]
    fn every_kind_of_event_serializes_with_its_own_kind() {
        let bodies = every_kind();
        assert_eq!(bodies.len(), 24);
        for body in bodies {
            let event = event(1, body);
            assert_eq!(to_json(&event)["kind"], json!(event.kind()));
        }
    }

    #[test]
    fn an_event_serializes_its_fixed_fields_with_a_utc_iso_8601_timestamp() {
        let json = to_json(&event(3, EventBody::AttemptStarted { step: charge(2) }));
        assert_eq!(
            json,
            json!({
                "kind": "attempt_started",
                "sequence": 3,
                "timestamp": "2023-11-14T22:13:20.123Z",
                "journey_id": "order-42",
                "workflow": "orders",
                "step": "charge",
                "attempt": 2,
            })
        );
    }

    #[test]
    fn a_step_hook_event_carries_the_policy_the_hook_and_the_triggering_attempt() {
        let json = to_json(&event(
            5,
            EventBody::HookCalled {
                hook: audit(HookName::OnStepRetry, Some(charge(1))),
                lifecycle: Some(Lifecycle::FailWorkflow(Reason::new("fraud"))),
            },
        ));
        assert_eq!(json["policy"], json!("audit"));
        assert_eq!(json["hook"], json!("on step retry"));
        assert_eq!(json["step"], json!("charge"));
        assert_eq!(json["attempt"], json!(1));
        assert_eq!(
            json["lifecycle"],
            json!({"FailWorkflow": {"code": "fraud", "message": null, "details": null}})
        );
    }

    #[test]
    fn data_emitted_by_a_step_is_serialized_as_the_value_itself() {
        let json = to_json(&event(
            4,
            EventBody::StepInfo {
                step: charge(1),
                message: "charging".to_string(),
                data: Some(AnyValue::new(vec![1_i64, 2])),
            },
        ));
        assert_eq!(json["message"], json!("charging"));
        assert_eq!(json["data"], json!([1, 2]));
    }

    #[test]
    fn a_decision_names_who_decided_it() {
        let by_default = to_json(&event(
            9,
            EventBody::JourneySucceeded {
                decided_by: DecidedBy::Default,
            },
        ));
        assert_eq!(by_default["decided_by"], json!("default"));

        let by_hook = to_json(&event(
            9,
            EventBody::JourneySucceeded {
                decided_by: DecidedBy::Hook(HookRef {
                    policy: "close".to_string(),
                    hook: HookName::OnStepSuccess,
                }),
            },
        ));
        assert_eq!(
            by_hook["decided_by"],
            json!({"policy": "close", "hook": "on step success"})
        );
    }

    #[test]
    fn an_abort_carries_its_reason_as_the_specification_writes_it_and_its_details() {
        let json = to_json(&event(
            6,
            EventBody::JourneyAborted {
                step: Some(charge(1)),
                reason: AbortReason::RequiredDataMissing,
                details: Some(AbortDetails::Data {
                    key: "price".to_string(),
                    requester: Requester::Adapter("pricing".to_string()),
                }),
            },
        ));
        assert_eq!(json["reason"], json!("required data missing"));
        assert_eq!(
            json["details"],
            json!({"data": {"key": "price", "requester": {"adapter": "pricing"}}})
        );
    }

    #[test]
    fn timestamps_are_rendered_in_utc_as_rfc_3339() {
        assert_eq!(utc(0, 0).unwrap(), "1970-01-01T00:00:00Z");
        assert_eq!(utc(951_868_800, 0).unwrap(), "2000-03-01T00:00:00Z");
        assert_eq!(utc(1_709_208_000, 0).unwrap(), "2024-02-29T12:00:00Z");
        assert_eq!(
            utc(1_700_000_000, 123_000_000).unwrap(),
            "2023-11-14T22:13:20.123Z"
        );
        assert_eq!(
            utc(1_700_000_000, 5).unwrap(),
            "2023-11-14T22:13:20.000000005Z"
        );
        assert_eq!(utc(253_402_300_799, 0).unwrap(), "9999-12-31T23:59:59Z");
        assert_eq!(utc(-62_167_219_200, 0).unwrap(), "0000-01-01T00:00:00Z");
    }

    #[test]
    fn a_timestamp_just_before_1970_borrows_from_the_previous_second() {
        let just_before = UNIX_EPOCH - Duration::from_nanos(100);
        assert_eq!(seconds_since_epoch(just_before), Some((-1, 999_999_900)));
        assert_eq!(
            rfc_3339(just_before).unwrap(),
            "1969-12-31T23:59:59.9999999Z"
        );
        assert_eq!(
            rfc_3339(UNIX_EPOCH - Duration::from_secs(1)).unwrap(),
            "1969-12-31T23:59:59Z"
        );
    }

    #[test]
    fn a_timestamp_outside_the_years_0_to_9999_cannot_be_rendered() {
        assert_eq!(utc(253_402_300_800, 0), None);
        assert_eq!(utc(-62_167_219_201, 0), None);
    }

    #[test]
    fn a_timestamp_before_1970_is_still_rendered() {
        let mut early = event(
            1,
            EventBody::JourneySucceeded {
                decided_by: DecidedBy::Default,
            },
        );
        early.timestamp = UNIX_EPOCH - Duration::from_secs(86_400);
        assert_eq!(to_json(&early)["timestamp"], json!("1969-12-31T00:00:00Z"));
    }
}
