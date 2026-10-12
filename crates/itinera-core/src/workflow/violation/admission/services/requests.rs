//! Admission's checks on the services each step and each rescue step requests.

use super::{counted, repeated};
use crate::service::{ProviderEntry, serving};
use crate::step::{ServiceNeed, StepDescriptor, StepName};
use crate::workflow::Violation;

/// Every violation in the services the steps and their rescue steps request: those requested
/// twice, then those no provider serves or that the provider serving them supplies as another
/// type.
pub(super) fn violations<W, M>(
    steps: &[StepDescriptor<W, M>],
    providers: &[Box<dyn ProviderEntry<W, M>>],
) -> Vec<Violation> {
    let requesting: Vec<&StepDescriptor<W, M>> = steps.iter().flat_map(requesting).collect();
    requesting
        .iter()
        .copied()
        .flat_map(services_requested_twice)
        .chain(
            requesting
                .iter()
                .flat_map(|step| unserved_and_mistyped(step, providers)),
        )
        .collect()
}

/// The step, then its rescue step, if it has one: each may request services, and a provider
/// serves each by its name.
fn requesting<W, M>(step: &StepDescriptor<W, M>) -> impl Iterator<Item = &StepDescriptor<W, M>> {
    std::iter::once(step).chain(step.rescue_step())
}

fn services_requested_twice<W, M>(step: &StepDescriptor<W, M>) -> impl Iterator<Item = Violation> {
    let name = step.name();
    counted(step.needs().services().iter().map(ServiceNeed::key))
        .into_iter()
        .filter(repeated)
        .map(move |(key, _)| service_requested_twice(name, key))
}

fn service_requested_twice(step: StepName, key: &'static str) -> Violation {
    Violation::ServiceRequestedTwice { step, key }
}

/// The services of a step that no provider serves, and those whose provider supplies another
/// type than the step requests.
fn unserved_and_mistyped<W, M>(
    step: &StepDescriptor<W, M>,
    providers: &[Box<dyn ProviderEntry<W, M>>],
) -> Vec<Violation> {
    let name = step.name();
    step.needs()
        .services()
        .iter()
        .filter_map(|need| served_wrongly(name, need, providers))
        .collect()
}

/// The violation of one service a step requests, if the workflow provides it wrongly.
fn served_wrongly<W, M>(
    step: StepName,
    need: &ServiceNeed,
    providers: &[Box<dyn ProviderEntry<W, M>>],
) -> Option<Violation> {
    let key = need.key();
    match serving(providers, step, key) {
        None => Some(Violation::ServiceNotProvided { step, key }),
        Some(provider) if provider.service_type() != need.service_type() => {
            Some(Violation::WrongServiceType {
                step,
                key,
                provider: provider.name(),
            })
        }
        Some(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::service::ProviderName;
    use crate::service::fixtures::PAYMENTS;
    use crate::workflow::WorkflowBuilder;
    use crate::workflow::fixtures::{Orders, orders};
    use crate::workflow::violation::admission::services::fixtures::{
        charging, numbered_payments, payments, rescued_charge, step, violations,
    };

    #[rstest]
    #[case::a_service_no_provider_supplies(
        charging(),
        Violation::ServiceNotProvided { step: StepName::new("charge"), key: "payments" },
    )]
    #[case::a_service_whose_only_provider_serves_another_step(
        charging()
            .step(step("refund", &[]))
            .provider(payments("refunds").step(StepName::new("refund"))),
        Violation::ServiceNotProvided { step: StepName::new("charge"), key: "payments" },
    )]
    #[case::one_key_requested_twice(
        orders()
            .step(step("charge", &[&PAYMENTS, &PAYMENTS]))
            .provider(payments("payments")),
        Violation::ServiceRequestedTwice { step: StepName::new("charge"), key: "payments" },
    )]
    #[case::a_provider_of_another_type_than_the_step_requests(
        charging().provider(numbered_payments()),
        Violation::WrongServiceType {
            step: StepName::new("charge"),
            key: "payments",
            provider: ProviderName::from("numbers"),
        },
    )]
    #[case::a_rescue_steps_service_only_a_provider_attached_to_the_rescued_step_supplies(
        rescued_charge(&[&PAYMENTS], &[&PAYMENTS])
            .provider(payments("charging").step(StepName::new("charge"))),
        Violation::ServiceNotProvided { step: StepName::new("charge_rescue"), key: "payments" },
    )]
    #[case::one_key_requested_twice_by_a_rescue_step(
        rescued_charge(&[], &[&PAYMENTS, &PAYMENTS]).provider(payments("payments")),
        Violation::ServiceRequestedTwice { step: StepName::new("charge_rescue"), key: "payments" },
    )]
    #[case::a_provider_attached_to_no_step_of_another_type_than_a_rescue_step_requests(
        rescued_charge(&[], &[&PAYMENTS]).provider(numbered_payments()),
        Violation::WrongServiceType {
            step: StepName::new("charge_rescue"),
            key: "payments",
            provider: ProviderName::from("numbers"),
        },
    )]
    #[case::a_provider_attached_to_a_rescue_step_of_another_type_than_it_requests(
        rescued_charge(&[], &[&PAYMENTS])
            .provider(payments("payments"))
            .provider(numbered_payments().step(StepName::new("charge_rescue"))),
        Violation::WrongServiceType {
            step: StepName::new("charge_rescue"),
            key: "payments",
            provider: ProviderName::from("numbers"),
        },
    )]
    fn a_workflow_serving_requested_services_wrongly_is_refused_with_its_violation(
        #[case] builder: WorkflowBuilder<Orders>,
        #[case] violation: Violation,
    ) {
        assert_eq!(violations(builder), [violation]);
    }

    #[test]
    fn a_provider_attached_to_a_step_replaces_the_one_attached_to_no_step_for_it() {
        let builder = charging()
            .step(step("ship", &[&PAYMENTS]))
            .provider(numbered_payments())
            .provider(payments("charging").step(StepName::new("charge")))
            .provider(payments("shipping").step(StepName::new("ship")));
        assert!(builder.build().is_ok());
    }

    #[test]
    fn a_rescue_step_is_served_by_the_provider_attached_to_no_step_not_by_the_rescued_steps() {
        let builder = rescued_charge(&[], &[&PAYMENTS])
            .provider(payments("payments"))
            .provider(numbered_payments().step(StepName::new("charge")));
        assert!(builder.build().is_ok());
    }
}
