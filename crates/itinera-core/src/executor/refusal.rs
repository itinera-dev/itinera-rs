use crate::error::Error;
use crate::policy::PolicyName;

/// Why `run` did not start a journey: the instance's initial global metadata sets a key only the
/// executor may set, or custom code called before the journey starts failed. There is no journey
/// and no event.
///
/// It names what refused the journey, and carries its error when custom code failed.
///
/// # Examples
///
/// ```
/// use itinera::executor::Refusal;
///
/// fn blames_the_dispatcher(refusal: &Refusal) -> bool {
///     matches!(refusal, Refusal::DispatcherFactory(_) | Refusal::Dispatcher(_))
/// }
/// ```
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Refusal {
    /// The instance's initial global metadata sets `journey_id`, which holds the journey ID and
    /// only the executor sets.
    #[error("the initial global metadata sets journey_id, which only the executor sets")]
    ReservedMetadataKey,
    /// A workflow policy failed while it was built for the journey.
    #[error("the workflow policy {policy} could not be built: {error}")]
    #[non_exhaustive]
    WorkflowPolicy {
        /// The policy's name.
        policy: PolicyName,
        /// The error it failed with.
        error: Error,
    },
    /// The dispatcher factory failed to create the journey's dispatcher.
    #[error("the dispatcher factory failed: {0}")]
    DispatcherFactory(Error),
    /// The dispatcher failed while the journey's reporters were added.
    #[error("the dispatcher failed while the reporters were added: {0}")]
    Dispatcher(Error),
    /// A reporter, or the dispatcher, failed while the journey ID or the instance's initial
    /// global metadata was set. The error is the reporter's, if one failed.
    #[error("the global metadata could not be set: {0}")]
    GlobalMetadata(Error),
}
