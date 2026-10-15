//! What a step hook returns, to keep the default or to decide what happens next with a lifecycle.

mod on_failure;
mod on_success;

pub use on_failure::OnFailure;
pub use on_success::OnSuccess;

use crate::step::Reason;

/// A lifecycle a hook returned, which decides what happens next, as the `hook_called` event
/// records it.
///
/// It displays as the lifecycle is named, for example `FailWorkflow`.
///
/// # Examples
///
/// ```
/// use itinera::policy::Lifecycle;
///
/// fn ends_the_journey(lifecycle: &Lifecycle) -> bool {
///     matches!(lifecycle, Lifecycle::FinishWorkflow | Lifecycle::FailWorkflow(_))
/// }
///
/// assert_eq!(Lifecycle::FinishWorkflow.to_string(), "FinishWorkflow");
/// ```
#[derive(Clone, Debug, derive_more::Display)]
#[non_exhaustive]
pub enum Lifecycle {
    /// The journey succeeds at once.
    FinishWorkflow,
    /// The journey fails at once, with this reason.
    #[display("FailWorkflow")]
    FailWorkflow(Reason),
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::finish_workflow(Lifecycle::FinishWorkflow, "FinishWorkflow")]
    #[case::fail_workflow(Lifecycle::FailWorkflow(Reason::new("fraud")), "FailWorkflow")]
    fn lifecycles_display_as_the_specification_writes_them(
        #[case] lifecycle: Lifecycle,
        #[case] written: &str,
    ) {
        assert_eq!(lifecycle.to_string(), written);
    }
}
