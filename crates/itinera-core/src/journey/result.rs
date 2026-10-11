//! What running a journey returns: its ID and how it ended.

use super::{Abort, DataBag, Failure, JourneyId};

/// What running a journey returns: a business outcome, with the journey's ID and how it ended.
///
/// It holds no step statuses, attempt counts or step names: those are in the event stream. It is
/// made only by itinera's executors, and is not `Clone`, since it may hold an [`Error`].
///
/// [`Error`]: crate::error::Error
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

#[cfg(test)]
mod tests {
    use std::fmt;

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
    #[case::status_succeeded(&StatusKind::Succeeded, "succeeded")]
    #[case::status_failed(&StatusKind::Failed, "failed")]
    #[case::status_aborted(&StatusKind::Aborted, "aborted")]
    fn statuses_display_as_the_specification_writes_them(
        #[case] name: &dyn fmt::Display,
        #[case] written: &str,
    ) {
        assert_eq!(name.to_string(), written);
    }
}
