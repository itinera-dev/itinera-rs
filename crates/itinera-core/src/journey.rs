//! Journeys, each one execution of a workflow: their identity, their data, and how they end.

use std::collections::BTreeMap;

use crate::error::Error;
use crate::policy::{StepHook, WorkflowHook};
use crate::step::Reason;
use crate::value::AnyValue;

/// The identifier of one journey, unique to it.
///
/// # Examples
///
/// ```
/// use itinera::journey::JourneyId;
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
    derive_more::Display,
    derive_more::From,
    derive_more::AsRef,
)]
#[from(forward)]
#[as_ref(forward)]
pub struct JourneyId(String);

/// Why a journey failed.
///
/// It displays as the cause is named, for example `retries exhausted`.
///
/// # Examples
///
/// ```
/// use itinera::journey::FailureCause;
///
/// assert_eq!(FailureCause::AbnormalTermination.to_string(), "abnormal termination");
/// assert_eq!(FailureCause::FailWorkflow.to_string(), "FailWorkflow");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::Display)]
#[non_exhaustive]
pub enum FailureCause {
    /// A step reported a failure that is not retriable.
    #[display("failure")]
    Failure,
    /// A step's retry budget was spent.
    #[display("retries exhausted")]
    RetriesExhausted,
    /// A step ended in an abnormal termination that is not retried.
    #[display("abnormal termination")]
    AbnormalTermination,
    /// A hook returned `FailWorkflow`.
    FailWorkflow,
}

/// What ended a step's last attempt when its retry budget was spent: the reason of a failure,
/// or the error that ended it abnormally.
///
/// Events hold the error as its message, the default `E`; the result holds the error itself.
///
/// # Examples
///
/// ```
/// use itinera::journey::LastFailure;
///
/// fn describe(last: &LastFailure) -> &str {
///     match last {
///         LastFailure::Reason(reason) => reason.code(),
///         LastFailure::Error(message) => message,
///     }
/// }
/// ```
#[derive(Clone, Debug)]
pub enum LastFailure<E = String> {
    /// The attempt reported a retriable failure, with this reason.
    Reason(Reason),
    /// The attempt ended in an abnormal termination, with this error.
    Error(E),
}

/// Why a journey was aborted.
///
/// It displays as the reason is named, for example `reporter failed`. The reasons
/// `invalid lifecycle` and `not a value` cannot happen in Rust, so they are absent.
///
/// # Examples
///
/// ```
/// use itinera::journey::AbortReason;
///
/// assert_eq!(AbortReason::ReporterFailed.to_string(), "reporter failed");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::Display)]
#[non_exhaustive]
pub enum AbortReason {
    /// A step's constructor, or the input adapter resolving one of its inputs, failed.
    #[display("step could not be built")]
    StepCouldNotBeBuilt,
    /// A step policy failed while being built for an attempt.
    #[display("policy could not be built")]
    PolicyCouldNotBeBuilt,
    /// A required request has no value.
    #[display("required data missing")]
    RequiredDataMissing,
    /// A requested value has the wrong type.
    #[display("wrong type")]
    WrongType,
    /// A hook, or a role operation it called, failed.
    #[display("hook failed")]
    HookFailed,
    /// A reporter, or a dispatcher while dispatching, failed.
    #[display("reporter failed")]
    ReporterFailed,
}

/// The data of one journey: a map from string keys to values.
///
/// It starts with the instance's initial data. At the end of a journey that succeeded or failed,
/// the result carries it as the journey's output. Its keys are listed in order.
///
/// # Examples
///
/// ```
/// use itinera::journey::DataBag;
/// use itinera::value::AnyValue;
///
/// fn amount(data: &DataBag) -> Option<i64> {
///     data.get("amount").and_then(as_amount)
/// }
///
/// fn as_amount(value: &AnyValue) -> Option<i64> {
///     value.downcast_ref().copied()
/// }
///
/// fn listed(data: &DataBag) -> Vec<&str> {
///     data.keys().collect()
/// }
/// ```
#[derive(Clone, Debug, derive_more::IntoIterator)]
#[into_iterator(owned, ref)]
pub struct DataBag {
    values: BTreeMap<String, AnyValue>,
}

impl DataBag {
    pub(crate) fn new() -> Self {
        Self {
            values: BTreeMap::new(),
        }
    }

    pub(crate) fn insert(&mut self, key: String, value: AnyValue) {
        self.values.insert(key, value);
    }

    /// The value under a key, if there is one.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::DataBag;
    ///
    /// fn customer(data: &DataBag) -> Option<&str> {
    ///     data.get("customer")?.downcast_ref::<String>().map(String::as_str)
    /// }
    /// ```
    pub fn get(&self, key: &str) -> Option<&AnyValue> {
        self.values.get(key)
    }

    /// The keys, in order.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::DataBag;
    ///
    /// fn has_amount(data: &DataBag) -> bool {
    ///     data.keys().any(is_amount)
    /// }
    ///
    /// fn is_amount(key: &str) -> bool {
    ///     key == "amount"
    /// }
    /// ```
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(String::as_str)
    }
}

/// What running a journey returns: a business outcome, with the journey's ID and how it ended.
///
/// It holds no step statuses, attempt counts or step names: those are in the event stream. It is
/// made only by itinera's executors, and is not `Clone`, since it may hold an [`Error`].
///
/// # Examples
///
/// ```
/// use itinera::journey::{JourneyResult, StatusKind};
///
/// fn summary(result: &JourneyResult) -> String {
///     format!("{}: {}", result.journey_id, result.status.kind())
/// }
///
/// fn succeeded(result: &JourneyResult) -> bool {
///     result.status.kind() == StatusKind::Succeeded
/// }
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub struct JourneyResult {
    /// The ID of the journey that ran.
    pub journey_id: JourneyId,
    /// How the journey ended, with what explains it.
    pub status: JourneyStatus,
}

/// How a journey ended, with what explains it, and its data bag when it has an output.
///
/// An aborted journey has no data bag: an abort is a fault, and its data may stand halfway through
/// a change.
///
/// # Examples
///
/// ```
/// use itinera::journey::{FailureCause, JourneyStatus};
///
/// fn describe(status: &JourneyStatus) -> String {
///     match status {
///         JourneyStatus::Succeeded { .. } => "succeeded".to_string(),
///         JourneyStatus::Failed { failure, .. } => format!("failed: {}", failure.cause()),
///         JourneyStatus::Aborted(abort) => format!("aborted: {}", abort.reason()),
///         _ => status.kind().to_string(),
///     }
/// }
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub enum JourneyStatus {
    /// Every step succeeded or was skipped, or a hook finished the workflow.
    #[non_exhaustive]
    Succeeded {
        /// The data bag as it stood at the end.
        data: DataBag,
    },
    /// A step failed, or a hook failed the workflow.
    #[non_exhaustive]
    Failed {
        /// Why it failed.
        failure: Failure,
        /// The data bag as it stood at the end.
        data: DataBag,
    },
    /// Something illegal happened, and the journey was stopped.
    Aborted(Abort),
}

impl JourneyStatus {
    /// The status alone, without what explains it.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::{JourneyStatus, StatusKind};
    ///
    /// fn has_output(status: &JourneyStatus) -> bool {
    ///     status.kind() != StatusKind::Aborted
    /// }
    /// ```
    pub fn kind(&self) -> StatusKind {
        match self {
            Self::Succeeded { .. } => StatusKind::Succeeded,
            Self::Failed { .. } => StatusKind::Failed,
            Self::Aborted(_) => StatusKind::Aborted,
        }
    }

    /// The data bag, for a journey that succeeded or failed; `None` for an aborted one.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::{DataBag, JourneyStatus};
    ///
    /// fn output_keys(status: &JourneyStatus) -> Vec<&str> {
    ///     status.data().map(keys).unwrap_or_default()
    /// }
    ///
    /// fn keys(data: &DataBag) -> Vec<&str> {
    ///     data.keys().collect()
    /// }
    /// ```
    pub fn data(&self) -> Option<&DataBag> {
        match self {
            Self::Succeeded { data } | Self::Failed { data, .. } => Some(data),
            Self::Aborted(_) => None,
        }
    }
}

/// A journey's final status, without what explains it.
///
/// It displays as the specification names it, for example `succeeded`.
///
/// # Examples
///
/// ```
/// use itinera::journey::StatusKind;
///
/// assert_eq!(StatusKind::Aborted.to_string(), "aborted");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::Display)]
#[non_exhaustive]
pub enum StatusKind {
    /// The journey succeeded.
    #[display("succeeded")]
    Succeeded,
    /// The journey failed.
    #[display("failed")]
    Failed,
    /// The journey was aborted.
    #[display("aborted")]
    Aborted,
}

/// Why a journey failed: one variant per cause, each holding the reason when a step or hook gave
/// one, and the error when an error ended the last attempt.
///
/// # Examples
///
/// ```
/// use itinera::journey::Failure;
///
/// fn code(failure: &Failure) -> Option<&str> {
///     match failure {
///         Failure::Failure(reason) | Failure::FailWorkflow(reason) => Some(reason.code()),
///         _ => None,
///     }
/// }
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub enum Failure {
    /// A step reported a failure that is not retriable, with this reason.
    Failure(Reason),
    /// A step's retry budget was spent, and this ended its last attempt.
    RetriesExhausted(LastFailure<Error>),
    /// A step ended in an abnormal termination that is not retried, with this error.
    AbnormalTermination(Error),
    /// A hook returned `FailWorkflow`, with this reason.
    FailWorkflow(Reason),
}

impl Failure {
    /// The failure's cause.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::{Failure, FailureCause};
    ///
    /// fn decided_by_a_hook(failure: &Failure) -> bool {
    ///     failure.cause() == FailureCause::FailWorkflow
    /// }
    /// ```
    pub fn cause(&self) -> FailureCause {
        match self {
            Self::Failure(_) => FailureCause::Failure,
            Self::RetriesExhausted(_) => FailureCause::RetriesExhausted,
            Self::AbnormalTermination(_) => FailureCause::AbnormalTermination,
            Self::FailWorkflow(_) => FailureCause::FailWorkflow,
        }
    }
}

/// Why a journey was aborted: one variant per abort reason, each holding its details, and the
/// error when failing custom code caused it.
///
/// # Examples
///
/// ```
/// use itinera::journey::Abort;
///
/// fn caused_by(abort: &Abort) -> Option<String> {
///     match abort {
///         Abort::HookFailed(error) | Abort::ReporterFailed(error) => Some(error.to_string()),
///         _ => None,
///     }
/// }
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub enum Abort {
    /// A step's constructor, or the input adapter resolving one of its inputs, failed with this
    /// error.
    StepCouldNotBeBuilt(Error),
    /// A step policy failed while being built for an attempt.
    #[non_exhaustive]
    PolicyCouldNotBeBuilt {
        /// The policy's name.
        policy: String,
        /// The error it failed with.
        error: Error,
    },
    /// A required request has no value.
    RequiredDataMissing(MissingData),
    /// A requested value has the wrong type.
    #[non_exhaustive]
    WrongType {
        /// The key that was requested.
        key: String,
        /// Who requested it.
        requester: Requester,
    },
    /// A hook, or a role operation it called, failed with this error.
    HookFailed(Error),
    /// A reporter, or a dispatcher while dispatching, failed with this error.
    ReporterFailed(Error),
}

impl Abort {
    /// The abort's reason.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::{Abort, AbortReason};
    ///
    /// fn caused_by_a_reporter(abort: &Abort) -> bool {
    ///     abort.reason() == AbortReason::ReporterFailed
    /// }
    /// ```
    pub fn reason(&self) -> AbortReason {
        match self {
            Self::StepCouldNotBeBuilt(_) => AbortReason::StepCouldNotBeBuilt,
            Self::PolicyCouldNotBeBuilt { .. } => AbortReason::PolicyCouldNotBeBuilt,
            Self::RequiredDataMissing(_) => AbortReason::RequiredDataMissing,
            Self::WrongType { .. } => AbortReason::WrongType,
            Self::HookFailed(_) => AbortReason::HookFailed,
            Self::ReporterFailed(_) => AbortReason::ReporterFailed,
        }
    }
}

/// What a required request found missing: data under a key, or the failure's reason or the error
/// that a step hook requested from the step it acts on.
///
/// It is the result's counterpart of [`event::MissingData`](crate::event::MissingData), without
/// the step's name, since the result names no step.
///
/// # Examples
///
/// ```
/// use itinera::journey::MissingData;
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
        /// Who requested it.
        requester: Requester,
    },
    /// The failure's reason, which the attempt did not report.
    #[non_exhaustive]
    Reason {
        /// The policy's name.
        policy: String,
        /// The hook that requested it.
        hook: StepHook,
    },
    /// The error, which did not end the attempt.
    #[non_exhaustive]
    Error {
        /// The policy's name.
        policy: String,
        /// The hook that requested it.
        hook: StepHook,
    },
}

/// Who made a request whose data could not be resolved: a step for one of its inputs, an input
/// adapter resolving a step's input, a step hook, or a workflow hook.
///
/// It is the result's counterpart of [`event::Requester`](crate::event::Requester), without the
/// step's name, since the result names no step.
///
/// # Examples
///
/// ```
/// use itinera::journey::Requester;
///
/// fn policy(requester: &Requester) -> Option<&str> {
///     match requester {
///         Requester::StepHook { policy, .. } | Requester::WorkflowHook { policy, .. } => {
///             Some(policy)
///         }
///         _ => None,
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Requester {
    /// A step, for one of its inputs.
    Step,
    /// An input adapter, for an input of a step.
    #[non_exhaustive]
    Adapter {
        /// The adapter's name.
        adapter: String,
    },
    /// A step hook.
    #[non_exhaustive]
    StepHook {
        /// The policy's name.
        policy: String,
        /// The hook.
        hook: StepHook,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn causes_and_abort_reasons_display_as_the_specification_writes_them() {
        assert_eq!(FailureCause::Failure.to_string(), "failure");
        assert_eq!(
            FailureCause::RetriesExhausted.to_string(),
            "retries exhausted"
        );
        assert_eq!(
            FailureCause::AbnormalTermination.to_string(),
            "abnormal termination"
        );
        assert_eq!(FailureCause::FailWorkflow.to_string(), "FailWorkflow");
        assert_eq!(
            AbortReason::StepCouldNotBeBuilt.to_string(),
            "step could not be built"
        );
        assert_eq!(
            AbortReason::PolicyCouldNotBeBuilt.to_string(),
            "policy could not be built"
        );
        assert_eq!(
            AbortReason::RequiredDataMissing.to_string(),
            "required data missing"
        );
        assert_eq!(AbortReason::WrongType.to_string(), "wrong type");
        assert_eq!(AbortReason::HookFailed.to_string(), "hook failed");
        assert_eq!(AbortReason::ReporterFailed.to_string(), "reporter failed");
    }

    fn bag() -> DataBag {
        let mut data = DataBag::new();
        data.insert("customer".to_string(), AnyValue::new("ana".to_string()));
        data.insert("amount".to_string(), AnyValue::new(42_i64));
        data
    }

    fn amount(value: &AnyValue) -> Option<i64> {
        value.downcast_ref().copied()
    }

    #[test]
    fn a_data_bag_lists_its_keys_in_order() {
        assert_eq!(bag().keys().collect::<Vec<_>>(), ["amount", "customer"]);
    }

    #[test]
    fn a_data_bag_gives_the_value_under_a_key() {
        let data = bag();
        assert_eq!(data.get("amount").and_then(amount), Some(42));
        assert!(data.get("missing").is_none());
    }

    #[test]
    fn a_data_bag_iterates_over_its_keys_and_values() {
        let keys: Vec<String> = bag().into_iter().map(|(key, _)| key).collect();
        assert_eq!(keys, ["amount", "customer"]);
    }

    #[test]
    fn only_a_journey_that_succeeded_or_failed_has_a_data_bag() {
        let succeeded = JourneyStatus::Succeeded { data: bag() };
        let failed = JourneyStatus::Failed {
            failure: Failure::Failure(Reason::new("declined")),
            data: bag(),
        };
        let aborted = JourneyStatus::Aborted(Abort::ReporterFailed(Error::msg("down")));

        assert!(succeeded.data().is_some());
        assert!(failed.data().is_some());
        assert!(aborted.data().is_none());
    }

    #[test]
    fn a_status_names_its_kind() {
        assert_eq!(
            JourneyStatus::Succeeded { data: bag() }.kind(),
            StatusKind::Succeeded
        );
        assert_eq!(
            JourneyStatus::Aborted(Abort::HookFailed(Error::msg("down"))).kind(),
            StatusKind::Aborted
        );
        assert_eq!(StatusKind::Succeeded.to_string(), "succeeded");
        assert_eq!(StatusKind::Failed.to_string(), "failed");
        assert_eq!(StatusKind::Aborted.to_string(), "aborted");
    }

    #[test]
    fn a_failure_names_its_cause() {
        assert_eq!(
            Failure::Failure(Reason::new("declined")).cause(),
            FailureCause::Failure
        );
        assert_eq!(
            Failure::RetriesExhausted(LastFailure::Error(Error::msg("timeout"))).cause(),
            FailureCause::RetriesExhausted
        );
        assert_eq!(
            Failure::AbnormalTermination(Error::msg("timeout")).cause(),
            FailureCause::AbnormalTermination
        );
        assert_eq!(
            Failure::FailWorkflow(Reason::new("fraud")).cause(),
            FailureCause::FailWorkflow
        );
    }

    #[test]
    fn an_abort_names_its_reason() {
        let missing = MissingData::Key {
            key: "amount".to_string(),
            requester: Requester::Step,
        };
        assert_eq!(
            Abort::StepCouldNotBeBuilt(Error::msg("closed")).reason(),
            AbortReason::StepCouldNotBeBuilt
        );
        assert_eq!(
            Abort::PolicyCouldNotBeBuilt {
                policy: "audit".to_string(),
                error: Error::msg("closed"),
            }
            .reason(),
            AbortReason::PolicyCouldNotBeBuilt
        );
        assert_eq!(
            Abort::RequiredDataMissing(missing).reason(),
            AbortReason::RequiredDataMissing
        );
        assert_eq!(
            Abort::WrongType {
                key: "amount".to_string(),
                requester: Requester::Step,
            }
            .reason(),
            AbortReason::WrongType
        );
        assert_eq!(
            Abort::HookFailed(Error::msg("closed")).reason(),
            AbortReason::HookFailed
        );
        assert_eq!(
            Abort::ReporterFailed(Error::msg("closed")).reason(),
            AbortReason::ReporterFailed
        );
    }
}
