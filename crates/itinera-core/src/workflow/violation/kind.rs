/// The kinds of violation a workflow can have when it is built.
///
/// It displays as the specification names the violation, for example `duplicate step name`. A
/// policy that needs a role the workflow does not provide, a part in a mode the executor does not
/// accept, and a service provider of journey scope that requests the step's name or the attempt
/// number do not compile, so they have no kind here.
///
/// # Examples
///
/// ```
/// use itinera::workflow::ViolationKind;
///
/// assert_eq!(ViolationKind::StepAdaptedTwice.to_string(), "step adapted twice");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::Display)]
#[non_exhaustive]
pub enum ViolationKind {
    /// Two policies attached to the same step, or to the workflow, define the same hook.
    #[display("hook defined twice")]
    HookDefinedTwice,
    /// Two steps, counting each step's rescue step, have the same name.
    #[display("duplicate step name")]
    DuplicateStepName,
    /// A step has both a rescue step and a retry budget.
    #[display("rescued step retried")]
    RescuedStepRetried,
    /// Two input adapters are attached to the same step.
    #[display("step adapted twice")]
    StepAdaptedTwice,
    /// An input adapter is attached to a step the workflow does not have.
    #[display("input adapter for unknown step")]
    InputAdapterForUnknownStep,
    /// A step requests a service that no provider serves for it.
    #[cfg(feature = "unstable")]
    #[display("service not provided")]
    ServiceNotProvided,
    /// A step requests one service twice.
    #[cfg(feature = "unstable")]
    #[display("service requested twice")]
    ServiceRequestedTwice,
    /// Two providers of one service are attached to the same step, or to no step.
    #[cfg(feature = "unstable")]
    #[display("service provided twice")]
    ServiceProvidedTwice,
    /// Two service providers have the same name.
    #[cfg(feature = "unstable")]
    #[display("duplicate provider name")]
    DuplicateProviderName,
    /// A service provider is attached to a step the workflow does not have.
    #[cfg(feature = "unstable")]
    #[display("service provider for unknown step")]
    ServiceProviderForUnknownStep,
    /// The provider serving a step supplies another type than the step requests.
    #[cfg(feature = "unstable")]
    #[display("wrong service type")]
    WrongServiceType,
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::hook_defined_twice(ViolationKind::HookDefinedTwice, "hook defined twice")]
    #[case::duplicate_step_name(ViolationKind::DuplicateStepName, "duplicate step name")]
    #[case::rescued_step_retried(ViolationKind::RescuedStepRetried, "rescued step retried")]
    #[case::step_adapted_twice(ViolationKind::StepAdaptedTwice, "step adapted twice")]
    #[case::input_adapter_for_unknown_step(
        ViolationKind::InputAdapterForUnknownStep,
        "input adapter for unknown step"
    )]
    #[cfg_attr(
        feature = "unstable",
        case::service_not_provided(ViolationKind::ServiceNotProvided, "service not provided")
    )]
    #[cfg_attr(
        feature = "unstable",
        case::service_requested_twice(
            ViolationKind::ServiceRequestedTwice,
            "service requested twice"
        )
    )]
    #[cfg_attr(
        feature = "unstable",
        case::service_provided_twice(
            ViolationKind::ServiceProvidedTwice,
            "service provided twice"
        )
    )]
    #[cfg_attr(
        feature = "unstable",
        case::duplicate_provider_name(
            ViolationKind::DuplicateProviderName,
            "duplicate provider name"
        )
    )]
    #[cfg_attr(
        feature = "unstable",
        case::service_provider_for_unknown_step(
            ViolationKind::ServiceProviderForUnknownStep,
            "service provider for unknown step"
        )
    )]
    #[cfg_attr(
        feature = "unstable",
        case::wrong_service_type(ViolationKind::WrongServiceType, "wrong service type")
    )]
    fn violation_kinds_display_as_the_specification_names_them(
        #[case] kind: ViolationKind,
        #[case] name: &str,
    ) {
        assert_eq!(kind.to_string(), name);
    }
}
