//! A journey's final status, without what explains it.

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
#[display(rename_all = "lowercase")]
#[non_exhaustive]
pub enum StatusKind {
    /// The journey succeeded.
    Succeeded,
    /// The journey failed.
    Failed,
    /// The journey was aborted.
    Aborted,
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::status_succeeded(StatusKind::Succeeded, "succeeded")]
    #[case::status_failed(StatusKind::Failed, "failed")]
    #[case::status_aborted(StatusKind::Aborted, "aborted")]
    fn statuses_display_as_the_specification_writes_them(
        #[case] name: StatusKind,
        #[case] written: &str,
    ) {
        assert_eq!(name.to_string(), written);
    }
}
