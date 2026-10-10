//! Journeys, each one execution of a workflow: their identity, their data, and how they end.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::error::Error;
use crate::policy::{PolicyName, StepHook, WorkflowHook};
use crate::step::Reason;
use crate::value::AnyValue;
use crate::workflow::AdapterName;

mod access;
mod contributor;

pub use access::{DataBagAccess, Read};
pub use contributor::Contributor;
pub(crate) use contributor::{Contribution, Contributions};

/// The identifier of one journey, unique to it. It is made when the journey's instance is
/// created, and cheap to clone.
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
pub struct JourneyId(Arc<str>);

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

    /// Puts a value under a key, and returns the value it replaced, if any.
    pub(crate) fn insert(&mut self, key: String, value: AnyValue) -> Option<AnyValue> {
        self.values.insert(key, value)
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
/// use itinera::journey::JourneyStatus;
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
        policy: PolicyName,
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
        policy: PolicyName,
        /// The hook that requested it.
        hook: StepHook,
    },
    /// The error, which did not end the attempt.
    #[non_exhaustive]
    Error {
        /// The policy's name.
        policy: PolicyName,
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
/// use itinera::policy::PolicyName;
///
/// fn policy(requester: &Requester) -> Option<PolicyName> {
///     match requester {
///         Requester::StepHook { policy, .. } | Requester::WorkflowHook { policy, .. } => {
///             Some(*policy)
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
        adapter: AdapterName,
    },
    /// A step hook.
    #[non_exhaustive]
    StepHook {
        /// The policy's name.
        policy: PolicyName,
        /// The hook.
        hook: StepHook,
    },
    /// A workflow hook.
    #[non_exhaustive]
    WorkflowHook {
        /// The policy's name.
        policy: PolicyName,
        /// The hook.
        hook: WorkflowHook,
    },
}

#[cfg(test)]
mod tests {
    use std::fmt;

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::cause_failure(&FailureCause::Failure, "failure")]
    #[case::cause_retries_exhausted(&FailureCause::RetriesExhausted, "retries exhausted")]
    #[case::cause_abnormal_termination(&FailureCause::AbnormalTermination, "abnormal termination")]
    #[case::cause_fail_workflow(&FailureCause::FailWorkflow, "FailWorkflow")]
    #[case::reason_step_could_not_be_built(&AbortReason::StepCouldNotBeBuilt, "step could not be built")]
    #[case::reason_policy_could_not_be_built(
        &AbortReason::PolicyCouldNotBeBuilt,
        "policy could not be built"
    )]
    #[case::reason_required_data_missing(&AbortReason::RequiredDataMissing, "required data missing")]
    #[case::reason_wrong_type(&AbortReason::WrongType, "wrong type")]
    #[case::reason_hook_failed(&AbortReason::HookFailed, "hook failed")]
    #[case::reason_reporter_failed(&AbortReason::ReporterFailed, "reporter failed")]
    #[case::status_succeeded(&StatusKind::Succeeded, "succeeded")]
    #[case::status_failed(&StatusKind::Failed, "failed")]
    #[case::status_aborted(&StatusKind::Aborted, "aborted")]
    fn statuses_causes_and_abort_reasons_display_as_the_specification_writes_them(
        #[case] name: &dyn fmt::Display,
        #[case] written: &str,
    ) {
        assert_eq!(name.to_string(), written);
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

    fn missing_amount() -> MissingData {
        MissingData::Key {
            key: "amount".to_string(),
            requester: Requester::Step,
        }
    }

    #[rstest]
    #[case::succeeded(JourneyStatus::Succeeded { data: bag() }, true)]
    #[case::failed(
        JourneyStatus::Failed {
            failure: Failure::Failure(Reason::new("declined")),
            data: bag(),
        },
        true
    )]
    #[case::aborted(
        JourneyStatus::Aborted(Abort::ReporterFailed(Error::msg("down"))),
        false
    )]
    fn only_a_journey_that_succeeded_or_failed_has_a_data_bag(
        #[case] status: JourneyStatus,
        #[case] has_data: bool,
    ) {
        assert_eq!(status.data().is_some(), has_data);
    }

    #[rstest]
    #[case::succeeded(JourneyStatus::Succeeded { data: bag() }, StatusKind::Succeeded)]
    #[case::failed(
        JourneyStatus::Failed {
            failure: Failure::Failure(Reason::new("declined")),
            data: bag(),
        },
        StatusKind::Failed
    )]
    #[case::aborted(
        JourneyStatus::Aborted(Abort::HookFailed(Error::msg("down"))),
        StatusKind::Aborted
    )]
    fn a_status_names_its_kind(#[case] status: JourneyStatus, #[case] kind: StatusKind) {
        assert_eq!(status.kind(), kind);
    }

    #[rstest]
    #[case::failure(Failure::Failure(Reason::new("declined")), FailureCause::Failure)]
    #[case::retries_exhausted(
        Failure::RetriesExhausted(LastFailure::Error(Error::msg("timeout"))),
        FailureCause::RetriesExhausted
    )]
    #[case::abnormal_termination(
        Failure::AbnormalTermination(Error::msg("timeout")),
        FailureCause::AbnormalTermination
    )]
    #[case::fail_workflow(
        Failure::FailWorkflow(Reason::new("fraud")),
        FailureCause::FailWorkflow
    )]
    fn a_failure_names_its_cause(#[case] failure: Failure, #[case] cause: FailureCause) {
        assert_eq!(failure.cause(), cause);
    }

    #[rstest]
    #[case::step_could_not_be_built(
        Abort::StepCouldNotBeBuilt(Error::msg("closed")),
        AbortReason::StepCouldNotBeBuilt
    )]
    #[case::policy_could_not_be_built(
        Abort::PolicyCouldNotBeBuilt {
            policy: PolicyName::from("audit"),
            error: Error::msg("closed"),
        },
        AbortReason::PolicyCouldNotBeBuilt
    )]
    #[case::required_data_missing(
        Abort::RequiredDataMissing(missing_amount()),
        AbortReason::RequiredDataMissing
    )]
    #[case::wrong_type(
        Abort::WrongType {
            key: "amount".to_string(),
            requester: Requester::Step,
        },
        AbortReason::WrongType
    )]
    #[case::hook_failed(Abort::HookFailed(Error::msg("closed")), AbortReason::HookFailed)]
    #[case::reporter_failed(
        Abort::ReporterFailed(Error::msg("closed")),
        AbortReason::ReporterFailed
    )]
    fn an_abort_names_its_reason(#[case] abort: Abort, #[case] reason: AbortReason) {
        assert_eq!(abort.reason(), reason);
    }
}
