use std::num::{NonZeroU32, NonZeroU64};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

use crate::value::{AnyValue, Value};

/// The identifier of one journey, unique to it.
///
/// # Examples
///
/// ```
/// use itinera::JourneyId;
///
/// let id = JourneyId::from("order-42");
/// let text: &str = id.as_ref();
/// assert_eq!(text, "order-42");
/// assert_eq!(id.to_string(), "order-42");
/// assert_eq!(JourneyId::from(String::from("order-42")), id);
/// ```
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    derive_more::Display,
    derive_more::From,
    derive_more::AsRef,
)]
#[serde(transparent)]
#[from(forward)]
#[as_ref(forward)]
pub struct JourneyId(String);

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

/// The name of a step hook, which acts on one attempt of a step.
///
/// It displays as the hook is named, for example `on step success`.
///
/// # Examples
///
/// ```
/// use itinera::StepHook;
///
/// assert_eq!(StepHook::OnStepRetry.to_string(), "on step retry");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub enum StepHook {
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
}

serde_plain::derive_display_from_serialize!(StepHook);

/// The name of a workflow hook, called once at the end of a journey.
///
/// It displays as the hook is named, for example `on workflow success`.
///
/// # Examples
///
/// ```
/// use itinera::WorkflowHook;
///
/// assert_eq!(WorkflowHook::OnWorkflowFailure.to_string(), "on workflow failure");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub enum WorkflowHook {
    /// Called once the journey has succeeded.
    #[serde(rename = "on workflow success")]
    OnWorkflowSuccess,
    /// Called once the journey has failed.
    #[serde(rename = "on workflow failure")]
    OnWorkflowFailure,
}

serde_plain::derive_display_from_serialize!(WorkflowHook);

/// The name of any hook: a step hook or a workflow hook.
///
/// It displays and serializes as the hook is named.
///
/// # Examples
///
/// ```
/// use itinera::{HookName, StepHook};
///
/// let hook = HookName::from(StepHook::OnStepFailure);
/// assert_eq!(hook.to_string(), "on step failure");
/// ```
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, derive_more::Display, derive_more::From,
)]
#[serde(untagged)]
#[non_exhaustive]
pub enum HookName {
    /// A step hook.
    Step(StepHook),
    /// A workflow hook.
    Workflow(WorkflowHook),
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

/// A hook that was called: a step hook, with the step and attempt that triggered it, or a
/// workflow hook.
///
/// It serializes as the policy, the hook, and for a step hook the step and attempt.
///
/// # Examples
///
/// ```
/// use itinera::HookSource;
///
/// fn triggered_by(source: &HookSource) -> Option<&str> {
///     match source {
///         HookSource::Step { step, .. } => Some(&step.step),
///         _ => None,
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum HookSource {
    /// A step hook.
    #[non_exhaustive]
    Step {
        /// The policy's name.
        policy: String,
        /// The hook.
        hook: StepHook,
        /// The step and attempt that triggered it.
        #[serde(flatten)]
        step: StepAttempt,
    },
    /// A workflow hook.
    #[non_exhaustive]
    Workflow {
        /// The policy's name.
        policy: String,
        /// The hook.
        hook: WorkflowHook,
    },
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

/// The policy and step hook whose returned lifecycle decided something.
///
/// Only step hooks return lifecycles, so only they decide. `H` is the set of hooks that can take
/// the decision: any step hook by default, or a narrower set such as [`GiveUpHook`].
///
/// # Examples
///
/// ```
/// use itinera::DecidingHook;
///
/// fn describe(hook: &DecidingHook) -> String {
///     format!("{}, {}", hook.policy, hook.hook)
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct DecidingHook<H = StepHook> {
    /// The policy's name.
    pub policy: String,
    /// The hook.
    pub hook: H,
}

/// A step hook that gives a step up when it returns `FailWorkflow`.
///
/// It displays as the hook is named, for example `on step retry`.
///
/// # Examples
///
/// ```
/// use itinera::GiveUpHook;
///
/// assert_eq!(GiveUpHook::OnStepRetry.to_string(), "on step retry");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub enum GiveUpHook {
    /// Called before a step is attempted again.
    #[serde(rename = "on step retry")]
    OnStepRetry,
    /// Called after an attempt ended in an abnormal termination.
    #[serde(rename = "on step abnormal termination")]
    OnStepAbnormalTermination,
}

serde_plain::derive_display_from_serialize!(GiveUpHook);

struct DecidedBy<'a, H>(Option<&'a DecidingHook<H>>);

impl<H: Serialize> Serialize for DecidedBy<'_, H> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            None => serializer.serialize_str("default"),
            Some(hook) => hook.serialize(serializer),
        }
    }
}

const BY_DEFAULT: DecidedBy<'static, StepHook> = DecidedBy(None);

fn finished_by<S: Serializer>(policy: &Option<String>, serializer: S) -> Result<S::Ok, S::Error> {
    match policy {
        None => BY_DEFAULT.serialize(serializer),
        Some(policy) => {
            let mut map = serializer.serialize_map(Some(2))?;
            map.serialize_entry("policy", policy)?;
            map.serialize_entry("hook", &StepHook::OnStepSuccess)?;
            map.end()
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

serde_plain::derive_display_from_serialize!(RetryCause);

fn retry_decision<S: Serializer>(cause: &RetryCause, serializer: S) -> Result<S::Ok, S::Error> {
    let mut map = serializer.serialize_map(Some(2))?;
    map.serialize_entry("cause", cause)?;
    map.serialize_entry("decided_by", &BY_DEFAULT)?;
    map.end()
}

/// Why a step will not be attempted again.
///
/// It displays as the cause is named, for example `retries exhausted`. It serializes as the
/// `cause` and `decided_by`: `"default"`, or the hook that returned `FailWorkflow`.
///
/// # Examples
///
/// ```
/// use itinera::GiveUpCause;
///
/// assert_eq!(GiveUpCause::RetriesExhausted.to_string(), "retries exhausted");
/// ```
#[derive(Clone, Debug, derive_more::Display)]
#[non_exhaustive]
pub enum GiveUpCause {
    /// The attempt reported a failure that is not retriable.
    #[display("failure")]
    Failure,
    /// The attempt ended in an abnormal termination, and the step does not allow retrying it.
    #[display("abnormal termination")]
    AbnormalTermination,
    /// The retry budget is spent.
    #[display("retries exhausted")]
    RetriesExhausted,
    /// A hook returned `FailWorkflow`.
    #[display("FailWorkflow")]
    #[non_exhaustive]
    FailWorkflow {
        /// The hook that returned it.
        decided_by: DecidingHook<GiveUpHook>,
        /// The reason it carried.
        reason: Reason,
    },
}

impl Serialize for GiveUpCause {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        match self {
            Self::FailWorkflow { decided_by, reason } => {
                map.serialize_entry("cause", &FailWorkflowCause(reason))?;
                map.serialize_entry("decided_by", &DecidedBy(Some(decided_by)))?;
            }
            Self::Failure | Self::AbnormalTermination | Self::RetriesExhausted => {
                map.serialize_entry("cause", &self.to_string())?;
                map.serialize_entry("decided_by", &BY_DEFAULT)?;
            }
        }
        map.end()
    }
}

struct FailWorkflowCause<'a>(&'a Reason);

impl Serialize for FailWorkflowCause<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("FailWorkflow", self.0)?;
        map.end()
    }
}

/// Why a journey failed.
///
/// It displays as the cause is named, for example `retries exhausted`.
///
/// # Examples
///
/// ```
/// use itinera::FailureCause;
///
/// assert_eq!(FailureCause::AbnormalTermination.to_string(), "abnormal termination");
/// assert_eq!(FailureCause::FailWorkflow.to_string(), "FailWorkflow");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub enum FailureCause {
    /// A step reported a failure that is not retriable.
    #[serde(rename = "failure")]
    Failure,
    /// A step's retry budget was spent.
    #[serde(rename = "retries exhausted")]
    RetriesExhausted,
    /// A step ended in an abnormal termination that is not retried.
    #[serde(rename = "abnormal termination")]
    AbnormalTermination,
    /// A hook returned `FailWorkflow`.
    FailWorkflow,
}

serde_plain::derive_display_from_serialize!(FailureCause);

/// What ended a step's last attempt when its retry budget was spent: the reason of a failure,
/// or the message of the error that ended it abnormally.
///
/// # Examples
///
/// ```
/// use itinera::LastFailure;
///
/// fn describe(last: &LastFailure) -> &str {
///     match last {
///         LastFailure::Reason(reason) => reason.code(),
///         LastFailure::Error(message) => message,
///     }
/// }
/// ```
#[derive(Clone, Debug)]
pub enum LastFailure {
    /// The attempt reported a retriable failure, with this reason.
    Reason(Reason),
    /// The attempt ended in an abnormal termination, with this error message.
    Error(String),
}

/// Why a journey failed, with what the cause carries and who decided it.
///
/// It serializes as the `cause`, the `reason` and the `error` message, each `null` when the cause
/// carries none, and `decided_by`: `"default"`, or the hook that returned `FailWorkflow`.
///
/// # Examples
///
/// ```
/// use itinera::{FailureCause, JourneyFailure};
///
/// fn by_a_hook(failure: &JourneyFailure) -> bool {
///     failure.cause() == FailureCause::FailWorkflow
/// }
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum JourneyFailure {
    /// The step reported a failure that is not retriable, with this reason.
    Failure(Reason),
    /// The step's retry budget was spent.
    RetriesExhausted(LastFailure),
    /// The step ended in an abnormal termination that is not retried, with this error message.
    AbnormalTermination(String),
    /// A hook of the step returned `FailWorkflow`.
    #[non_exhaustive]
    FailWorkflow {
        /// The hook that returned it.
        decided_by: DecidingHook,
        /// The reason it carried.
        reason: Reason,
    },
}

impl JourneyFailure {
    /// The failure's cause.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::{FailureCause, JourneyFailure};
    ///
    /// fn retried(failure: &JourneyFailure) -> bool {
    ///     failure.cause() == FailureCause::RetriesExhausted
    /// }
    /// ```
    pub fn cause(&self) -> FailureCause {
        match self {
            Self::Failure(_) => FailureCause::Failure,
            Self::RetriesExhausted(_) => FailureCause::RetriesExhausted,
            Self::AbnormalTermination(_) => FailureCause::AbnormalTermination,
            Self::FailWorkflow { .. } => FailureCause::FailWorkflow,
        }
    }
}

impl Serialize for JourneyFailure {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (reason, error, hook) = match self {
            Self::Failure(reason) | Self::RetriesExhausted(LastFailure::Reason(reason)) => {
                (Some(reason), None, None)
            }
            Self::RetriesExhausted(LastFailure::Error(error))
            | Self::AbnormalTermination(error) => (None, Some(error), None),
            Self::FailWorkflow { decided_by, reason } => (Some(reason), None, Some(decided_by)),
        };
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("cause", &self.cause())?;
        map.serialize_entry("reason", &reason)?;
        map.serialize_entry("error", &error)?;
        map.serialize_entry("decided_by", &DecidedBy(hook))?;
        map.end()
    }
}

/// Why a journey was aborted.
///
/// It displays as the reason is named, for example `reporter failed`. The reasons
/// `invalid lifecycle` and `not a value` cannot happen in Rust, so they are absent.
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

serde_plain::derive_display_from_serialize!(AbortReason);

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

/// Who made an optional request that had no value: a step for one of its inputs, a hook, or an
/// input adapter resolving a step's input; with the step and attempt it was made for, except for
/// a workflow hook.
///
/// It serializes as the step and attempt, and the policy and hook or the adapter when the request
/// is theirs.
///
/// # Examples
///
/// ```
/// use itinera::RequestSource;
///
/// fn adapter(source: &RequestSource) -> Option<&str> {
///     match source {
///         RequestSource::Adapter { adapter, .. } => Some(adapter),
///         _ => None,
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum RequestSource {
    /// A step, for one of its inputs, during this attempt.
    Step(StepAttempt),
    /// A hook.
    Hook(HookSource),
    /// An input adapter, for an input of this step and attempt.
    #[non_exhaustive]
    Adapter {
        /// The adapter's name.
        adapter: String,
        /// The step and attempt whose input it was resolving.
        #[serde(flatten)]
        step: StepAttempt,
    },
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
/// It serializes as one map holding `kind`, `sequence`, `timestamp` in ISO 8601 in UTC, `journey_id`,
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
        /// The key that was requested.
        key: String,
        /// Who requested it.
        #[serde(flatten)]
        requester: RequestSource,
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
        /// The hook, and the step and attempt that triggered a step hook.
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
        /// The name of the step during which it happened, if any.
        step: Option<String>,
        /// Why it was aborted.
        reason: AbortReason,
        /// What it concerns, where the reason needs more.
        details: Option<AbortDetails>,
        /// The message of the error that caused it, when failing custom code did.
        error: Option<String>,
    },
    /// The executor decided to attempt a step again.
    #[non_exhaustive]
    StepRetrying {
        /// The step and the attempt that failed.
        #[serde(flatten)]
        step: StepAttempt,
        /// Why it will be attempted again. The executor's own rule always decides it.
        #[serde(flatten, serialize_with = "retry_decision")]
        cause: RetryCause,
    },
    /// The executor decided not to attempt a step again: it ends as failed.
    #[non_exhaustive]
    StepGivenUp {
        /// The step and its last attempt.
        #[serde(flatten)]
        step: StepAttempt,
        /// Why it will not be attempted again, and who decided it.
        #[serde(flatten)]
        cause: GiveUpCause,
    },
    /// The executor decided that the journey succeeds.
    #[non_exhaustive]
    JourneySucceeded {
        /// The policy whose `on step success` returned `FinishWorkflow`, or `None` when no
        /// step was left. It serializes as `"default"`, or as the policy and hook.
        #[serde(serialize_with = "finished_by")]
        decided_by: Option<String>,
    },
    /// The executor decided that the journey fails.
    #[non_exhaustive]
    JourneyFailed {
        /// The name of the step that failed, or whose hook returned `FailWorkflow`.
        step: String,
        /// Why the journey failed, and who decided it.
        #[serde(flatten)]
        failure: JourneyFailure,
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
        /// The hook, and the step and attempt that triggered a step hook.
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
        /// The hook, and the step and attempt that triggered a step hook.
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
        /// The hook, and the step and attempt that triggered a step hook.
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
            journey_id: JourneyId::from("order-42"),
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

    fn audit_step(hook: StepHook, step: StepAttempt) -> HookSource {
        HookSource::Step {
            policy: "audit".to_string(),
            hook,
            step,
        }
    }

    fn audit_workflow(hook: WorkflowHook) -> HookSource {
        HookSource::Workflow {
            policy: "audit".to_string(),
            hook,
        }
    }

    fn close(hook: StepHook) -> DecidingHook {
        DecidingHook {
            policy: "close".to_string(),
            hook,
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
                key: "discount".to_string(),
                requester: RequestSource::Step(charge(1)),
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
                hook: audit_step(StepHook::OnStepSuccess, charge(1)),
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
                error: Some("disk full".to_string()),
            },
            EventBody::StepRetrying {
                step: charge(1),
                cause: RetryCause::RetriableFailure,
            },
            EventBody::StepGivenUp {
                step: charge(2),
                cause: GiveUpCause::RetriesExhausted,
            },
            EventBody::JourneySucceeded { decided_by: None },
            EventBody::JourneyFailed {
                step: "charge".to_string(),
                failure: JourneyFailure::RetriesExhausted(LastFailure::Reason(reason())),
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
                hook: audit_workflow(WorkflowHook::OnWorkflowSuccess),
                message: "done".to_string(),
                data: None,
            },
            EventBody::JourneyWarning {
                hook: audit_workflow(WorkflowHook::OnWorkflowSuccess),
                message: "late".to_string(),
                data: None,
            },
            EventBody::JourneyError {
                hook: audit_workflow(WorkflowHook::OnWorkflowFailure),
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
                hook: audit_step(StepHook::OnStepRetry, charge(1)),
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
    fn a_workflow_hook_event_carries_no_step() {
        let json = to_json(&event(
            12,
            EventBody::JourneyInfo {
                hook: audit_workflow(WorkflowHook::OnWorkflowSuccess),
                message: "done".to_string(),
                data: None,
            },
        ));
        assert_eq!(json["policy"], json!("audit"));
        assert_eq!(json["hook"], json!("on workflow success"));
        assert!(json.get("step").is_none());
        assert!(json.get("attempt").is_none());
    }

    #[test]
    fn a_decision_names_who_decided_it() {
        let by_default = to_json(&event(9, EventBody::JourneySucceeded { decided_by: None }));
        assert_eq!(by_default["decided_by"], json!("default"));

        let by_hook = to_json(&event(
            9,
            EventBody::JourneySucceeded {
                decided_by: Some("close".to_string()),
            },
        ));
        assert_eq!(
            by_hook["decided_by"],
            json!({"policy": "close", "hook": "on step success"})
        );
    }

    #[test]
    fn a_retry_is_always_decided_by_default() {
        let json = to_json(&event(
            7,
            EventBody::StepRetrying {
                step: charge(1),
                cause: RetryCause::AbnormalTermination,
            },
        ));
        assert_eq!(json["cause"], json!("abnormal termination"));
        assert_eq!(json["decided_by"], json!("default"));
    }

    #[test]
    fn a_step_given_up_by_fail_workflow_is_decided_by_the_hook_that_returned_it() {
        let json = to_json(&event(
            8,
            EventBody::StepGivenUp {
                step: charge(2),
                cause: GiveUpCause::FailWorkflow {
                    decided_by: DecidingHook {
                        policy: "close".to_string(),
                        hook: GiveUpHook::OnStepRetry,
                    },
                    reason: Reason::new("fraud"),
                },
            },
        ));
        assert_eq!(
            json["cause"],
            json!({"FailWorkflow": {"code": "fraud", "message": null, "details": null}})
        );
        assert_eq!(
            json["decided_by"],
            json!({"policy": "close", "hook": "on step retry"})
        );

        let by_default = to_json(&event(
            8,
            EventBody::StepGivenUp {
                step: charge(2),
                cause: GiveUpCause::Failure,
            },
        ));
        assert_eq!(by_default["cause"], json!("failure"));
        assert_eq!(by_default["decided_by"], json!("default"));
    }

    #[test]
    fn a_journey_failed_after_retries_carries_the_last_attempts_error_message() {
        let json = to_json(&event(
            10,
            EventBody::JourneyFailed {
                step: "charge".to_string(),
                failure: JourneyFailure::RetriesExhausted(LastFailure::Error(
                    "timeout".to_string(),
                )),
            },
        ));
        assert_eq!(json["step"], json!("charge"));
        assert!(json.get("attempt").is_none());
        assert_eq!(json["cause"], json!("retries exhausted"));
        assert_eq!(json["reason"], json!(null));
        assert_eq!(json["error"], json!("timeout"));
        assert_eq!(json["decided_by"], json!("default"));
    }

    #[test]
    fn a_journey_failed_by_fail_workflow_carries_the_reason_and_the_hook() {
        let json = to_json(&event(
            10,
            EventBody::JourneyFailed {
                step: "charge".to_string(),
                failure: JourneyFailure::FailWorkflow {
                    decided_by: close(StepHook::OnStepSuccess),
                    reason: Reason::new("fraud"),
                },
            },
        ));
        assert_eq!(json["cause"], json!("FailWorkflow"));
        assert_eq!(json["reason"]["code"], json!("fraud"));
        assert_eq!(json["error"], json!(null));
        assert_eq!(
            json["decided_by"],
            json!({"policy": "close", "hook": "on step success"})
        );
    }

    #[test]
    fn an_optional_request_names_its_requester_and_the_step_it_was_for() {
        let by_adapter = to_json(&event(
            2,
            EventBody::OptionalInputAbsent {
                key: "discount".to_string(),
                requester: RequestSource::Adapter {
                    adapter: "pricing".to_string(),
                    step: charge(1),
                },
            },
        ));
        assert_eq!(by_adapter["key"], json!("discount"));
        assert_eq!(by_adapter["adapter"], json!("pricing"));
        assert_eq!(by_adapter["step"], json!("charge"));
        assert_eq!(by_adapter["attempt"], json!(1));

        let by_workflow_hook = to_json(&event(
            11,
            EventBody::OptionalInputAbsent {
                key: "discount".to_string(),
                requester: RequestSource::Hook(audit_workflow(WorkflowHook::OnWorkflowFailure)),
            },
        ));
        assert_eq!(by_workflow_hook["policy"], json!("audit"));
        assert_eq!(by_workflow_hook["hook"], json!("on workflow failure"));
        assert!(by_workflow_hook.get("step").is_none());
    }

    #[test]
    fn any_hook_displays_as_it_is_named() {
        assert_eq!(
            HookName::from(StepHook::OnStepAbnormalTermination).to_string(),
            "on step abnormal termination"
        );
        assert_eq!(
            HookName::from(WorkflowHook::OnWorkflowSuccess).to_string(),
            "on workflow success"
        );
        assert_eq!(
            serde_json::to_value(HookName::from(WorkflowHook::OnWorkflowSuccess)).unwrap(),
            json!("on workflow success")
        );
    }

    #[test]
    fn an_abort_carries_its_reason_by_name_and_its_details() {
        let json = to_json(&event(
            6,
            EventBody::JourneyAborted {
                step: Some("charge".to_string()),
                reason: AbortReason::RequiredDataMissing,
                details: Some(AbortDetails::Data {
                    key: "price".to_string(),
                    requester: Requester::Adapter("pricing".to_string()),
                }),
                error: None,
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
        let mut early = event(1, EventBody::JourneySucceeded { decided_by: None });
        early.timestamp = UNIX_EPOCH - Duration::from_secs(86_400);
        assert_eq!(to_json(&early)["timestamp"], json!("1969-12-31T00:00:00Z"));
    }
}
