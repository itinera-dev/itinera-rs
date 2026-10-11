use super::AdapterName;
use crate::policy::{PolicyName, StepHook, WorkflowHook};
#[cfg(feature = "unstable")]
use crate::service::ProviderName;
use crate::step::StepName;

mod admission;
mod kind;
mod violations;

pub(super) use admission::check;
pub use kind::ViolationKind;
pub use violations::Violations;

/// One configuration error in how a workflow is put together, found when it is built.
///
/// It displays as the violation is named, followed by what it concerns, for example
/// `duplicate step name: charge`.
///
/// # Examples
///
/// ```
/// use itinera::workflow::{Violation, ViolationKind};
///
/// fn about_steps(violation: &Violation) -> bool {
///     violation.kind() == ViolationKind::DuplicateStepName
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, derive_more::Display)]
#[non_exhaustive]
pub enum Violation {
    /// Two or more steps have this name, counting each step's rescue step.
    #[display("duplicate step name: {step}")]
    #[non_exhaustive]
    DuplicateStepName {
        /// The name the steps share.
        step: StepName,
    },
    /// A step has both a rescue step and a retry budget, though a step with a rescue step is
    /// never retried.
    #[display("rescued step retried: {step}")]
    #[non_exhaustive]
    RescuedStepRetried {
        /// The step.
        step: StepName,
    },
    /// Two or more step policies attached to one step define the same hook.
    #[display("hook defined twice: {hook} on step {step}")]
    #[non_exhaustive]
    StepHookDefinedTwice {
        /// The step the policies are attached to.
        step: StepName,
        /// The hook they all define.
        hook: StepHook,
        /// The policies that define it, in the order they are attached.
        policies: Vec<PolicyName>,
    },
    /// Two or more workflow policies define the same hook.
    #[display("hook defined twice: {hook}")]
    #[non_exhaustive]
    WorkflowHookDefinedTwice {
        /// The hook they all define.
        hook: WorkflowHook,
        /// The policies that define it, in the order they are attached.
        policies: Vec<PolicyName>,
    },
    /// Two or more input adapters are attached to one step.
    #[display("step adapted twice: {step}")]
    #[non_exhaustive]
    StepAdaptedTwice {
        /// The step they are attached to.
        step: StepName,
        /// The adapters, in the order they were declared.
        adapters: Vec<AdapterName>,
    },
    /// An input adapter is attached to a step the workflow does not have.
    #[display("input adapter for unknown step: {adapter} on {step}")]
    #[non_exhaustive]
    InputAdapterForUnknownStep {
        /// The adapter.
        adapter: AdapterName,
        /// The name it gives for a step, which no step of the workflow has.
        step: StepName,
    },
    /// A step requests a service that no provider serves for it.
    #[cfg(feature = "unstable")]
    #[display("service not provided: {key} for {step}")]
    #[non_exhaustive]
    ServiceNotProvided {
        /// The step.
        step: StepName,
        /// The key of the service it requests.
        key: &'static str,
    },
    /// A step requests one service twice or more.
    #[cfg(feature = "unstable")]
    #[display("service requested twice: {key} by {step}")]
    #[non_exhaustive]
    ServiceRequestedTwice {
        /// The step.
        step: StepName,
        /// The key it requests more than once.
        key: &'static str,
    },
    /// Two or more providers of one service are attached to the same step, or to no step.
    #[cfg(feature = "unstable")]
    #[display("service provided twice: {key}{}", for_step(step))]
    #[non_exhaustive]
    ServiceProvidedTwice {
        /// The key of the service they supply.
        key: &'static str,
        /// The step they are all attached to, or `None` when none of them is attached to a step.
        step: Option<StepName>,
        /// The providers, in the order they were declared.
        providers: Vec<ProviderName>,
    },
    /// Two or more service providers have this name.
    #[cfg(feature = "unstable")]
    #[display("duplicate provider name: {provider}")]
    #[non_exhaustive]
    DuplicateProviderName {
        /// The name the providers share.
        provider: ProviderName,
    },
    /// A service provider is attached to a step the workflow does not have.
    #[cfg(feature = "unstable")]
    #[display("service provider for unknown step: {provider} on {step}")]
    #[non_exhaustive]
    ServiceProviderForUnknownStep {
        /// The provider.
        provider: ProviderName,
        /// The name it gives for a step, which no step of the workflow has.
        step: StepName,
    },
    /// The provider serving a step supplies another type than the step requests.
    #[cfg(feature = "unstable")]
    #[display("wrong service type: {provider} for {key} of {step}")]
    #[non_exhaustive]
    WrongServiceType {
        /// The step.
        step: StepName,
        /// The key of the service it requests.
        key: &'static str,
        /// The provider that serves it.
        provider: ProviderName,
    },
}

impl Violation {
    /// The violation's kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::workflow::{Violation, ViolationKind};
    ///
    /// fn about_hooks(violation: &Violation) -> bool {
    ///     violation.kind() == ViolationKind::HookDefinedTwice
    /// }
    /// ```
    pub fn kind(&self) -> ViolationKind {
        match self {
            Self::DuplicateStepName { .. } => ViolationKind::DuplicateStepName,
            Self::RescuedStepRetried { .. } => ViolationKind::RescuedStepRetried,
            Self::StepHookDefinedTwice { .. } | Self::WorkflowHookDefinedTwice { .. } => {
                ViolationKind::HookDefinedTwice
            }
            Self::StepAdaptedTwice { .. } => ViolationKind::StepAdaptedTwice,
            Self::InputAdapterForUnknownStep { .. } => ViolationKind::InputAdapterForUnknownStep,
            #[cfg(feature = "unstable")]
            Self::ServiceNotProvided { .. } => ViolationKind::ServiceNotProvided,
            #[cfg(feature = "unstable")]
            Self::ServiceRequestedTwice { .. } => ViolationKind::ServiceRequestedTwice,
            #[cfg(feature = "unstable")]
            Self::ServiceProvidedTwice { .. } => ViolationKind::ServiceProvidedTwice,
            #[cfg(feature = "unstable")]
            Self::DuplicateProviderName { .. } => ViolationKind::DuplicateProviderName,
            #[cfg(feature = "unstable")]
            Self::ServiceProviderForUnknownStep { .. } => {
                ViolationKind::ServiceProviderForUnknownStep
            }
            #[cfg(feature = "unstable")]
            Self::WrongServiceType { .. } => ViolationKind::WrongServiceType,
        }
    }
}

/// What follows the key of a service provided twice: the step the providers are attached to.
#[cfg(feature = "unstable")]
fn for_step(step: &Option<StepName>) -> String {
    match step {
        Some(step) => format!(" for {step}"),
        None => String::new(),
    }
}

#[cfg(all(test, feature = "unstable"))]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::to_no_step(None, "service provided twice: payments")]
    #[case::to_one_step(
        Some(StepName::new("charge")),
        "service provided twice: payments for charge"
    )]
    fn providers_of_one_service_display_the_step_they_are_attached_to(
        #[case] step: Option<StepName>,
        #[case] text: &str,
    ) {
        let violation = Violation::ServiceProvidedTwice {
            key: "payments",
            step,
            providers: Vec::new(),
        };
        assert_eq!(violation.to_string(), text);
    }
}
