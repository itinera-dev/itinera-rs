//! How a journey ended, with what explains it.

use super::StatusKind;
use crate::journey::{Abort, DataBag, Failure};

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

    /// Whether the journey succeeded.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::JourneyResult;
    ///
    /// fn report(result: &JourneyResult) -> &str {
    ///     if result.status.is_succeeded() { "done" } else { "not done" }
    /// }
    /// ```
    pub fn is_succeeded(&self) -> bool {
        matches!(self, Self::Succeeded { .. })
    }

    /// Whether the journey failed.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::JourneyStatus;
    ///
    /// fn needs_a_refund(status: &JourneyStatus) -> bool {
    ///     status.is_failed()
    /// }
    /// ```
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Whether the journey was aborted.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::JourneyStatus;
    ///
    /// fn needs_a_developer(status: &JourneyStatus) -> bool {
    ///     status.is_aborted()
    /// }
    /// ```
    pub fn is_aborted(&self) -> bool {
        matches!(self, Self::Aborted(_))
    }

    /// Why the journey failed, for a journey that failed; `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::{Failure, JourneyStatus};
    ///
    /// fn failure_code(status: &JourneyStatus) -> Option<&str> {
    ///     status.failure().and_then(Failure::reason).map(|reason| reason.code())
    /// }
    /// ```
    pub fn failure(&self) -> Option<&Failure> {
        match self {
            Self::Failed { failure, .. } => Some(failure),
            Self::Succeeded { .. } | Self::Aborted(_) => None,
        }
    }

    /// Why the journey was aborted, for an aborted journey; `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::{Abort, AbortReason, JourneyStatus};
    ///
    /// fn abort_reason(status: &JourneyStatus) -> Option<AbortReason> {
    ///     status.abort().map(Abort::reason)
    /// }
    /// ```
    pub fn abort(&self) -> Option<&Abort> {
        match self {
            Self::Aborted(abort) => Some(abort),
            Self::Succeeded { .. } | Self::Failed { .. } => None,
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::error::Error;
    use crate::journey::fixtures::bag;
    use crate::step::Reason;

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
    #[case::succeeded(succeeded(), StatusKind::Succeeded)]
    #[case::failed(failed(), StatusKind::Failed)]
    #[case::aborted(aborted(), StatusKind::Aborted)]
    fn a_status_answers_true_only_for_its_own_kind(
        #[case] status: JourneyStatus,
        #[case] kind: StatusKind,
    ) {
        let answers = [
            status.is_succeeded(),
            status.is_failed(),
            status.is_aborted(),
        ];
        let expected = [
            kind == StatusKind::Succeeded,
            kind == StatusKind::Failed,
            kind == StatusKind::Aborted,
        ];
        assert_eq!(answers, expected);
    }

    #[rstest]
    #[case::succeeded(succeeded(), None)]
    #[case::failed(failed(), Some("declined"))]
    #[case::aborted(aborted(), None)]
    fn only_a_failed_journey_has_a_failure(
        #[case] status: JourneyStatus,
        #[case] code: Option<&str>,
    ) {
        let reason = status.failure().and_then(Failure::reason);
        assert_eq!(reason.map(Reason::code), code);
    }

    #[rstest]
    #[case::succeeded(succeeded(), false)]
    #[case::failed(failed(), false)]
    #[case::aborted(aborted(), true)]
    fn only_an_aborted_journey_has_an_abort(#[case] status: JourneyStatus, #[case] has: bool) {
        assert_eq!(status.abort().is_some(), has);
    }

    fn succeeded() -> JourneyStatus {
        JourneyStatus::Succeeded { data: bag() }
    }

    fn failed() -> JourneyStatus {
        JourneyStatus::Failed {
            failure: Failure::Failure(Reason::new("declined")),
            data: bag(),
        }
    }

    fn aborted() -> JourneyStatus {
        JourneyStatus::Aborted(Abort::ReporterFailed(Error::msg("down")))
    }
}
