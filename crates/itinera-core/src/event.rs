//! Events: the record of what happens in a journey, and what only events carry.

use std::fmt;
use std::num::NonZeroU64;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::journey::{AbortReason, FailureCause, JourneyId, LastFailure};
use crate::policy::{Lifecycle, StepHook, WorkflowHook};
use crate::step::{Reason, StepAttempt};
use crate::value::AnyValue;

/// A hook that was called: a step hook, with the step and attempt that triggered it, or a
/// workflow hook.
///
/// # Examples
///
/// ```
/// use itinera::event::HookSource;
///
/// fn triggered_by(source: &HookSource) -> Option<&str> {
///     match source {
///         HookSource::Step { step, .. } => Some(&step.step),
///         _ => None,
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

/// The policy and step hook whose returned lifecycle decided something.
///
/// Only step hooks return lifecycles, so only they decide. `H` is the set of hooks that can take
/// the decision: any step hook by default, or a narrower set such as [`GiveUpHook`].
///
/// # Examples
///
/// ```
/// use itinera::event::DecidingHook;
///
/// fn describe(hook: &DecidingHook) -> String {
///     format!("{}, {}", hook.policy, hook.hook)
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
/// use itinera::event::GiveUpHook;
///
/// assert_eq!(GiveUpHook::OnStepRetry.to_string(), "on step retry");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::Display)]
#[non_exhaustive]
pub enum GiveUpHook {
    /// Called before a step is attempted again.
    #[display("on step retry")]
    OnStepRetry,
    /// Called after an attempt ended in an abnormal termination.
    #[display("on step abnormal termination")]
    OnStepAbnormalTermination,
}

/// Why a step will be attempted again.
///
/// It displays as the cause is named, for example `retriable failure`.
///
/// # Examples
///
/// ```
/// use itinera::event::RetryCause;
///
/// assert_eq!(RetryCause::RetriableFailure.to_string(), "retriable failure");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::Display)]
#[non_exhaustive]
pub enum RetryCause {
    /// The attempt reported a retriable failure.
    #[display("retriable failure")]
    RetriableFailure,
    /// The attempt ended in an abnormal termination, and the step allows retrying it.
    #[display("abnormal termination")]
    AbnormalTermination,
}

/// Why a step will not be attempted again.
///
/// It displays as the cause is named, for example `retries exhausted`. Every cause but
/// `FailWorkflow` is decided by default.
///
/// # Examples
///
/// ```
/// use itinera::event::GiveUpCause;
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

/// Why a journey failed, with what the cause carries and who decided it.
///
/// # Examples
///
/// ```
/// use itinera::event::JourneyFailure;
/// use itinera::journey::FailureCause;
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
    /// use itinera::event::JourneyFailure;
    /// use itinera::journey::FailureCause;
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

/// Who made a request whose data could not be resolved, and the step it was made for: a step for
/// one of its inputs, an input adapter resolving a step's input, a step hook, or a workflow hook,
/// which has no step.
///
/// # Examples
///
/// ```
/// use itinera::event::Requester;
///
/// fn adapter(requester: &Requester) -> Option<&str> {
///     match requester {
///         Requester::Adapter { adapter, .. } => Some(adapter),
///         _ => None,
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Requester {
    /// A step, for one of its inputs.
    #[non_exhaustive]
    Step {
        /// The step's name.
        step: String,
    },
    /// An input adapter, for an input of a step.
    #[non_exhaustive]
    Adapter {
        /// The adapter's name.
        adapter: String,
        /// The name of the step whose input it was resolving.
        step: String,
    },
    /// A step hook.
    #[non_exhaustive]
    StepHook {
        /// The policy's name.
        policy: String,
        /// The hook.
        hook: StepHook,
        /// The name of the step it acts on.
        step: String,
    },
    /// A workflow hook.
    #[non_exhaustive]
    WorkflowHook {
        /// The policy's name.
        policy: String,
        /// The hook.
        hook: WorkflowHook,
    },
}
impl Requester {
    fn step(&self) -> Option<&str> {
        match self {
            Self::Step { step } | Self::Adapter { step, .. } | Self::StepHook { step, .. } => {
                Some(step)
            }
            Self::WorkflowHook { .. } => None,
        }
    }
}

/// Who made an optional request that had no value: a step for one of its inputs, a hook, or an
/// input adapter resolving a step's input; with the step and attempt it was made for, except for
/// a workflow hook.
///
/// # Examples
///
/// ```
/// use itinera::event::RequestSource;
///
/// fn adapter(source: &RequestSource) -> Option<&str> {
///     match source {
///         RequestSource::Adapter { adapter, .. } => Some(adapter),
///         _ => None,
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
        step: StepAttempt,
    },
}

/// Why a journey was aborted, with exactly what that reason carries: the step during which it
/// happened, when there was one, its details, and the message of the error when failing custom
/// code caused it.
///
/// # Examples
///
/// ```
/// use itinera::event::JourneyAbort;
///
/// fn describe(abort: &JourneyAbort) -> String {
///     match abort.error() {
///         Some(error) => format!("{}: {error}", abort.reason()),
///         None => abort.reason().to_string(),
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum JourneyAbort {
    /// A step's constructor, or the input adapter resolving one of its inputs, failed.
    #[non_exhaustive]
    StepCouldNotBeBuilt {
        /// The step's name.
        step: String,
        /// The error's message.
        error: String,
    },
    /// A step policy failed while being built for an attempt.
    #[non_exhaustive]
    PolicyCouldNotBeBuilt {
        /// The step's name.
        step: String,
        /// The policy's name.
        policy: String,
        /// The error's message.
        error: String,
    },
    /// A required request has no value.
    #[non_exhaustive]
    RequiredDataMissing {
        /// What was requested, and by whom.
        missing: MissingData,
    },
    /// A requested value has the wrong type.
    #[non_exhaustive]
    WrongType {
        /// The key that was requested.
        key: String,
        /// Who requested it, and for which step.
        requester: Requester,
    },
    /// A hook, or a role operation it called, failed.
    #[non_exhaustive]
    HookFailed {
        /// The name of the step a step hook acts on; `None` for a workflow hook.
        step: Option<String>,
        /// The error's message.
        error: String,
    },
    /// A reporter, or a dispatcher while dispatching, failed.
    #[non_exhaustive]
    ReporterFailed {
        /// The name of the step during which it happened, if any.
        step: Option<String>,
        /// The error's message.
        error: String,
    },
}
impl JourneyAbort {
    /// The abort's reason.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::event::JourneyAbort;
    /// use itinera::journey::AbortReason;
    ///
    /// fn caused_by_a_reporter(abort: &JourneyAbort) -> bool {
    ///     abort.reason() == AbortReason::ReporterFailed
    /// }
    /// ```
    pub fn reason(&self) -> AbortReason {
        match self {
            Self::StepCouldNotBeBuilt { .. } => AbortReason::StepCouldNotBeBuilt,
            Self::PolicyCouldNotBeBuilt { .. } => AbortReason::PolicyCouldNotBeBuilt,
            Self::RequiredDataMissing { .. } => AbortReason::RequiredDataMissing,
            Self::WrongType { .. } => AbortReason::WrongType,
            Self::HookFailed { .. } => AbortReason::HookFailed,
            Self::ReporterFailed { .. } => AbortReason::ReporterFailed,
        }
    }

    /// The name of the step during which the journey was aborted, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::event::JourneyAbort;
    ///
    /// fn during_a_step(abort: &JourneyAbort) -> bool {
    ///     abort.step().is_some()
    /// }
    /// ```
    pub fn step(&self) -> Option<&str> {
        match self {
            Self::StepCouldNotBeBuilt { step, .. } | Self::PolicyCouldNotBeBuilt { step, .. } => {
                Some(step)
            }
            Self::RequiredDataMissing { missing } => missing.step(),
            Self::WrongType { requester, .. } => requester.step(),
            Self::HookFailed { step, .. } | Self::ReporterFailed { step, .. } => step.as_deref(),
        }
    }

    /// The message of the error that caused the abort, when failing custom code did.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::event::JourneyAbort;
    ///
    /// fn caused_by_failing_code(abort: &JourneyAbort) -> bool {
    ///     abort.error().is_some()
    /// }
    /// ```
    pub fn error(&self) -> Option<&str> {
        match self {
            Self::StepCouldNotBeBuilt { error, .. }
            | Self::PolicyCouldNotBeBuilt { error, .. }
            | Self::HookFailed { error, .. }
            | Self::ReporterFailed { error, .. } => Some(error),
            Self::RequiredDataMissing { .. } | Self::WrongType { .. } => None,
        }
    }
}

/// What a required request found missing: data under a key, or the failure's reason or the error
/// that a step hook requested from the step it acts on.
///
/// # Examples
///
/// ```
/// use itinera::event::MissingData;
///
/// fn key(missing: &MissingData) -> Option<&str> {
///     match missing {
///         MissingData::Key { key, .. } => Some(key),
///         _ => None,
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MissingData {
    /// Data under a key.
    #[non_exhaustive]
    Key {
        /// The key that was requested.
        key: String,
        /// Who requested it, and for which step.
        requester: Requester,
    },
    /// The failure's reason, which the attempt did not report.
    #[non_exhaustive]
    Reason {
        /// The policy's name.
        policy: String,
        /// The hook that requested it.
        hook: StepHook,
        /// The name of the step it acts on.
        step: String,
    },
    /// The error, which did not end the attempt.
    #[non_exhaustive]
    Error {
        /// The policy's name.
        policy: String,
        /// The hook that requested it.
        hook: StepHook,
        /// The name of the step it acts on.
        step: String,
    },
}
impl MissingData {
    fn step(&self) -> Option<&str> {
        match self {
            Self::Key { requester, .. } => requester.step(),
            Self::Reason { step, .. } | Self::Error { step, .. } => Some(step),
        }
    }
}

/// Who made a contribution: a step, or a hook.
///
/// # Examples
///
/// ```
/// use itinera::event::Source;
///
/// fn from_a_hook(source: &Source) -> bool {
///     matches!(source, Source::Hook(_))
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Source {
    /// A step, during this attempt.
    Step(StepAttempt),
    /// A hook.
    Hook(HookSource),
}

/// When an event was emitted: an instant, displayed in UTC in ISO 8601.
///
/// It displays to the nanosecond, without trailing zeros, for example
/// `2023-11-14T22:13:20.123Z`. A year outside 0 to 9999 is written as an ISO 8601 expanded year,
/// with its sign and at least five digits, for example `+10000-01-01T00:00:00Z`.
///
/// # Examples
///
/// ```
/// use std::time::{Duration, SystemTime, UNIX_EPOCH};
///
/// use itinera::event::Timestamp;
///
/// let timestamp = Timestamp::from(UNIX_EPOCH + Duration::from_millis(1_700_000_000_123));
/// assert_eq!(timestamp.to_string(), "2023-11-14T22:13:20.123Z");
/// let instant: SystemTime = timestamp.into();
/// assert_eq!(instant, UNIX_EPOCH + Duration::from_millis(1_700_000_000_123));
/// ```
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::From, derive_more::Into,
)]
pub struct Timestamp {
    instant: SystemTime,
}
const SECONDS_PER_DAY: i128 = 86_400;
const NANOS_PER_SECOND: u32 = 1_000_000_000;
impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (seconds, nanos) = seconds_since_epoch(self.instant);
        Utc { seconds, nanos }.fmt(f)
    }
}
struct Utc {
    seconds: i128,
    nanos: u32,
}
impl fmt::Display for Utc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { seconds, nanos } = *self;
        let (year, month, day) = civil_date(seconds.div_euclid(SECONDS_PER_DAY));
        if (0..=9999).contains(&year) {
            write!(f, "{year:04}")?;
        } else {
            write!(f, "{year:+06}")?;
        }
        let of_day = seconds.rem_euclid(SECONDS_PER_DAY);
        write!(
            f,
            "-{month:02}-{day:02}T{:02}:{:02}:{:02}",
            of_day / 3600,
            of_day % 3600 / 60,
            of_day % 60
        )?;
        if nanos > 0 {
            let (mut fraction, mut digits) = (nanos, 9);
            while fraction % 10 == 0 {
                fraction /= 10;
                digits -= 1;
            }
            write!(f, ".{fraction:0digits$}")?;
        }
        f.write_str("Z")
    }
}
fn seconds_since_epoch(timestamp: SystemTime) -> (i128, u32) {
    match timestamp.duration_since(UNIX_EPOCH) {
        Ok(after) => (i128::from(after.as_secs()), after.subsec_nanos()),
        Err(before) => {
            let before = before.duration();
            let seconds = -i128::from(before.as_secs());
            match before.subsec_nanos() {
                0 => (seconds, 0),
                nanos => (seconds - 1, NANOS_PER_SECOND - nanos),
            }
        }
    }
}
// Howard Hinnant's days-to-civil algorithm, for the proleptic Gregorian calendar.
fn civil_date(days_since_epoch: i128) -> (i128, i128, i128) {
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
    let year = year_of_era + era * 400 + i128::from(month <= 2);
    (year, month, day)
}

/// One event of a journey's event stream.
///
/// Events are made only by itinera's executors. Every event carries a sequence number,
/// increasing from 1 within the journey, a timestamp, the journey ID and the workflow name;
/// what else it carries depends on its [`body`](Event::body).
///
/// Events carry no format of their own: each reporter writes them as it chooses.
///
/// # Examples
///
/// ```
/// use itinera::event::{Event, EventBody};
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
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Event {
    /// The event's position in the journey's event stream, from 1.
    pub sequence: NonZeroU64,
    /// When the event was emitted.
    pub timestamp: Timestamp,
    /// The journey's ID.
    pub journey_id: JourneyId,
    /// The workflow's name.
    pub workflow: String,
    /// What the event says.
    pub body: EventBody,
}
impl Event {
    /// The event's kind, in snake_case, for example `step_failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::event::Event;
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
        adapter: String,
    },
    /// An input adapter failed for a step's input.
    #[non_exhaustive]
    InputAdapterFailed {
        /// The step and attempt.
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
        decided_by: Option<String>,
    },
    /// The executor decided that the journey fails.
    #[non_exhaustive]
    JourneyFailed {
        /// The name of the step that failed, or whose hook returned `FailWorkflow`.
        step: String,
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

#[cfg(test)]
pub(crate) mod tests {
    use std::num::NonZeroU32;
    use std::time::Duration;

    use rstest::rstest;

    use super::*;

    pub(crate) fn event(sequence: u64, body: EventBody) -> Event {
        Event {
            sequence: NonZeroU64::new(sequence).unwrap(),
            timestamp: Timestamp::from(UNIX_EPOCH + Duration::from_millis(1_700_000_000_123)),
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
                abort: JourneyAbort::ReporterFailed {
                    step: None,
                    error: "disk full".to_string(),
                },
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

    #[test]
    fn every_kind_of_event_has_its_own_kind() {
        let kinds: std::collections::HashSet<_> =
            every_kind().iter().map(EventBody::kind).collect();
        assert_eq!(kinds.len(), 24);
        let event = event(1, EventBody::AttemptStarted { step: charge(1) });
        assert_eq!(event.kind(), "attempt_started");
    }

    #[rstest]
    #[case::give_up_on_step_retry(&GiveUpHook::OnStepRetry, "on step retry")]
    #[case::give_up_on_step_abnormal_termination(
        &GiveUpHook::OnStepAbnormalTermination,
        "on step abnormal termination"
    )]
    #[case::retry_after_a_retriable_failure(&RetryCause::RetriableFailure, "retriable failure")]
    #[case::retry_after_an_abnormal_termination(
        &RetryCause::AbnormalTermination,
        "abnormal termination"
    )]
    #[case::given_up_after_a_failure(&GiveUpCause::Failure, "failure")]
    #[case::given_up_after_an_abnormal_termination(
        &GiveUpCause::AbnormalTermination,
        "abnormal termination"
    )]
    #[case::given_up_with_retries_exhausted(&GiveUpCause::RetriesExhausted, "retries exhausted")]
    fn names_display_as_the_specification_writes_them(
        #[case] name: &dyn fmt::Display,
        #[case] written: &str,
    ) {
        assert_eq!(name.to_string(), written);
    }

    #[test]
    fn a_step_given_up_by_fail_workflow_names_fail_workflow_as_its_cause() {
        let cause = GiveUpCause::FailWorkflow {
            decided_by: DecidingHook {
                policy: "close".to_string(),
                hook: GiveUpHook::OnStepRetry,
            },
            reason: Reason::new("fraud"),
        };
        assert_eq!(cause.to_string(), "FailWorkflow");
    }

    #[rstest]
    #[case::failure(
        JourneyFailure::Failure(Reason::new("declined")),
        FailureCause::Failure
    )]
    #[case::retries_exhausted(
        JourneyFailure::RetriesExhausted(LastFailure::Error("timeout".to_string())),
        FailureCause::RetriesExhausted
    )]
    #[case::abnormal_termination(
        JourneyFailure::AbnormalTermination("boom".to_string()),
        FailureCause::AbnormalTermination
    )]
    #[case::fail_workflow(
        JourneyFailure::FailWorkflow {
            decided_by: close(StepHook::OnStepSuccess),
            reason: Reason::new("fraud"),
        },
        FailureCause::FailWorkflow
    )]
    fn a_journey_failure_names_its_cause(
        #[case] failure: JourneyFailure,
        #[case] cause: FailureCause,
    ) {
        assert_eq!(failure.cause(), cause);
    }

    #[test]
    fn an_abort_while_data_was_resolved_names_the_step_it_was_for_but_no_error() {
        let abort = JourneyAbort::RequiredDataMissing {
            missing: MissingData::Key {
                key: "price".to_string(),
                requester: Requester::Adapter {
                    adapter: "pricing".to_string(),
                    step: "charge".to_string(),
                },
            },
        };
        assert_eq!(abort.reason(), AbortReason::RequiredDataMissing);
        assert_eq!(abort.step(), Some("charge"));
        assert_eq!(abort.error(), None);
    }

    #[test]
    fn a_required_request_for_a_missing_reason_names_the_step_the_hook_acts_on() {
        let abort = JourneyAbort::RequiredDataMissing {
            missing: MissingData::Reason {
                policy: "alarm".to_string(),
                hook: StepHook::OnStepFailure,
                step: "charge".to_string(),
            },
        };
        assert_eq!(abort.step(), Some("charge"));
        assert_eq!(abort.error(), None);
    }

    #[test]
    fn an_abort_by_a_workflow_hooks_request_names_no_step() {
        let abort = JourneyAbort::WrongType {
            key: "amount".to_string(),
            requester: Requester::WorkflowHook {
                policy: "close".to_string(),
                hook: WorkflowHook::OnWorkflowSuccess,
            },
        };
        assert_eq!(abort.reason(), AbortReason::WrongType);
        assert_eq!(abort.step(), None);
    }

    #[test]
    fn an_abort_caused_by_failing_code_carries_the_errors_message() {
        let abort = JourneyAbort::PolicyCouldNotBeBuilt {
            step: "ship".to_string(),
            policy: "broken".to_string(),
            error: "no configuration".to_string(),
        };
        assert_eq!(abort.reason(), AbortReason::PolicyCouldNotBeBuilt);
        assert_eq!(abort.step(), Some("ship"));
        assert_eq!(abort.error(), Some("no configuration"));
    }

    fn at(seconds: i128, nanos: u32) -> String {
        Utc { seconds, nanos }.to_string()
    }

    #[rstest]
    #[case::the_epoch(0, 0, "1970-01-01T00:00:00Z")]
    #[case::the_day_after_a_leap_day_in_a_century(951_868_800, 0, "2000-03-01T00:00:00Z")]
    #[case::a_leap_day(1_709_208_000, 0, "2024-02-29T12:00:00Z")]
    #[case::milliseconds(1_700_000_000, 123_000_000, "2023-11-14T22:13:20.123Z")]
    #[case::nanoseconds(1_700_000_000, 5, "2023-11-14T22:13:20.000000005Z")]
    #[case::the_last_second_of_year_9999(253_402_300_799, 0, "9999-12-31T23:59:59Z")]
    #[case::the_first_second_of_year_0(-62_167_219_200, 0, "0000-01-01T00:00:00Z")]
    #[case::a_day_before_the_epoch(-86_400, 0, "1969-12-31T00:00:00Z")]
    fn a_timestamp_displays_in_utc_in_iso_8601(
        #[case] seconds: i128,
        #[case] nanos: u32,
        #[case] written: &str,
    ) {
        assert_eq!(at(seconds, nanos), written);
    }

    #[test]
    fn a_timestamp_just_before_1970_borrows_from_the_previous_second() {
        let just_before = UNIX_EPOCH - Duration::from_nanos(100);
        assert_eq!(seconds_since_epoch(just_before), (-1, 999_999_900));
        assert_eq!(
            Timestamp::from(just_before).to_string(),
            "1969-12-31T23:59:59.9999999Z"
        );
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - Duration::from_secs(1)).to_string(),
            "1969-12-31T23:59:59Z"
        );
    }

    #[rstest]
    #[case::year_10000(253_402_300_800, "+10000-01-01T00:00:00Z")]
    #[case::year_minus_1(-62_167_219_201, "-00001-12-31T23:59:59Z")]
    fn a_year_outside_0_to_9999_is_written_as_an_expanded_year(
        #[case] seconds: i128,
        #[case] written: &str,
    ) {
        assert_eq!(at(seconds, 0), written);
    }
}
