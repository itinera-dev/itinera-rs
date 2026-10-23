//! Admission's checks on the services a workflow's steps request and its providers supply.

use super::{counted, grouped, is_known, is_unknown, more_than_once, repeated};
use crate::service::{ProviderEntry, ProviderName};
use crate::step::{StepDescriptor, StepName};
use crate::workflow::Violation;

#[cfg(test)]
mod fixtures;
mod requests;

/// Every violation in the services the steps and their rescue steps request and the providers
/// supply.
pub(super) fn violations<W, M>(
    steps: &[StepDescriptor<W, M>],
    providers: &[Box<dyn ProviderEntry<W, M>>],
) -> Vec<Violation> {
    requests::violations(steps, providers)
        .into_iter()
        .chain(duplicate_provider_names(providers))
        .chain(providers_for_unknown_steps(steps, providers))
        .chain(services_provided_twice(steps, providers))
        .collect()
}

fn duplicate_provider_names<W, M>(
    providers: &[Box<dyn ProviderEntry<W, M>>],
) -> impl Iterator<Item = Violation> {
    counted(providers.iter().map(|provider| provider.name()))
        .into_iter()
        .filter(repeated)
        .map(duplicate_provider_name)
}

fn duplicate_provider_name((provider, _): (ProviderName, usize)) -> Violation {
    Violation::DuplicateProviderName { provider }
}

fn providers_for_unknown_steps<'a, W, M>(
    steps: &'a [StepDescriptor<W, M>],
    providers: &'a [Box<dyn ProviderEntry<W, M>>],
) -> impl Iterator<Item = Violation> + 'a {
    providers
        .iter()
        .map(Box::as_ref)
        .flat_map(attachments)
        .filter(move |attachment| is_unknown(steps, attachment.step))
        .map(provider_for_unknown_step)
}

fn provider_for_unknown_step(attachment: Attachment) -> Violation {
    Violation::ServiceProviderForUnknownStep {
        provider: attachment.provider,
        step: attachment.step,
    }
}

/// Two or more providers of one service attached to no step, or to the same step of the
/// workflow.
fn services_provided_twice<W, M>(
    steps: &[StepDescriptor<W, M>],
    providers: &[Box<dyn ProviderEntry<W, M>>],
) -> impl Iterator<Item = Violation> {
    let unattached = providers
        .iter()
        .map(Box::as_ref)
        .filter(is_unattached)
        .map(unattached_by_key);
    let attached = providers
        .iter()
        .map(Box::as_ref)
        .flat_map(attachments)
        .filter(|attachment| is_known(steps, attachment.step))
        .map(attached_by_key);
    grouped(unattached.chain(attached))
        .into_iter()
        .filter(more_than_once)
        .map(service_provided_twice)
}

fn is_unattached<W, M>(provider: &&dyn ProviderEntry<W, M>) -> bool {
    provider.steps().is_empty()
}

fn unattached_by_key<W, M>(
    provider: &dyn ProviderEntry<W, M>,
) -> ((&'static str, Option<StepName>), ProviderName) {
    ((provider.key(), None), provider.name())
}

fn attached_by_key(attachment: Attachment) -> ((&'static str, Option<StepName>), ProviderName) {
    ((attachment.key, Some(attachment.step)), attachment.provider)
}

fn service_provided_twice(
    ((key, step), providers): ((&'static str, Option<StepName>), Vec<ProviderName>),
) -> Violation {
    Violation::ServiceProvidedTwice {
        key,
        step,
        providers,
    }
}

/// A provider attached to a step.
struct Attachment {
    provider: ProviderName,
    key: &'static str,
    step: StepName,
}

/// Each step the provider is attached to.
fn attachments<W, M>(provider: &dyn ProviderEntry<W, M>) -> impl Iterator<Item = Attachment> + '_ {
    provider
        .steps()
        .iter()
        .map(|step| attachment(provider, *step))
}

fn attachment<W, M>(provider: &dyn ProviderEntry<W, M>, step: StepName) -> Attachment {
    Attachment {
        provider: provider.name(),
        key: provider.key(),
        step,
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::service::fixtures::{Made, PAYMENTS, STOCK, per_attempt};
    use crate::workflow::WorkflowBuilder;
    use crate::workflow::fixtures::{Orders, adapter};
    use fixtures::{charge_requesting_nothing, charging, payments, rescued_charge, violations};

    #[rstest]
    #[case::two_providers_attached_to_no_step_though_no_step_requests_them(
        charge_requesting_nothing()
            .provider(payments("payments"))
            .provider(payments("backup")),
        Violation::ServiceProvidedTwice {
            key: "payments",
            step: None,
            providers: vec![ProviderName::from("payments"), ProviderName::from("backup")],
        },
    )]
    #[case::two_providers_attached_to_one_step(
        charging()
            .provider(payments("payments").step(StepName::new("charge")))
            .provider(payments("backup").step(StepName::new("charge"))),
        Violation::ServiceProvidedTwice {
            key: "payments",
            step: Some(StepName::new("charge")),
            providers: vec![ProviderName::from("payments"), ProviderName::from("backup")],
        },
    )]
    #[case::one_provider_attached_to_one_step_twice(
        charging().provider(
            payments("payments")
                .step(StepName::new("charge"))
                .step(StepName::new("charge")),
        ),
        Violation::ServiceProvidedTwice {
            key: "payments",
            step: Some(StepName::new("charge")),
            providers: vec![ProviderName::from("payments"), ProviderName::from("payments")],
        },
    )]
    #[case::one_provider_attached_to_one_rescue_step_twice(
        rescued_charge(&[], &[&PAYMENTS]).provider(
            payments("main")
                .step(StepName::new("charge_rescue"))
                .step(StepName::new("charge_rescue")),
        ),
        Violation::ServiceProvidedTwice {
            key: "payments",
            step: Some(StepName::new("charge_rescue")),
            providers: vec![ProviderName::from("main"), ProviderName::from("main")],
        },
    )]
    #[case::two_providers_with_one_name(
        charge_requesting_nothing()
            .provider(payments("shared"))
            .provider(per_attempt("shared", &STOCK, &Made::default())),
        Violation::DuplicateProviderName { provider: ProviderName::from("shared") },
    )]
    #[case::a_provider_attached_to_a_step_the_workflow_does_not_have(
        charge_requesting_nothing()
            .provider(payments("refunds").step(StepName::new("refund"))),
        Violation::ServiceProviderForUnknownStep {
            provider: ProviderName::from("refunds"),
            step: StepName::new("refund"),
        },
    )]
    #[case::a_provider_attached_to_the_rescue_step_of_a_step_without_one(
        charge_requesting_nothing()
            .provider(payments("payments").step(StepName::new("charge_rescue"))),
        Violation::ServiceProviderForUnknownStep {
            provider: ProviderName::from("payments"),
            step: StepName::new("charge_rescue"),
        },
    )]
    #[case::two_providers_attached_to_one_rescue_step(
        rescued_charge(&[], &[&PAYMENTS])
            .provider(payments("main").step(StepName::new("charge_rescue")))
            .provider(payments("backup").step(StepName::new("charge_rescue"))),
        Violation::ServiceProvidedTwice {
            key: "payments",
            step: Some(StepName::new("charge_rescue")),
            providers: vec![ProviderName::from("main"), ProviderName::from("backup")],
        },
    )]
    fn a_workflow_whose_providers_are_declared_wrongly_is_refused_with_its_violation(
        #[case] builder: WorkflowBuilder<Orders>,
        #[case] violation: Violation,
    ) {
        assert_eq!(violations(builder), [violation]);
    }

    #[test]
    fn an_input_adapter_and_a_service_provider_may_share_a_name() {
        let builder = charging()
            .provider(payments("pricing"))
            .input_adapter(adapter("pricing", "charge"));
        assert!(builder.build().is_ok());
    }

    #[test]
    fn a_provider_attached_to_a_step_that_does_not_request_its_service_is_allowed() {
        let builder = charge_requesting_nothing()
            .provider(payments("payments").step(StepName::new("charge")));
        assert!(builder.build().is_ok());
    }

    #[test]
    fn a_provider_attached_to_a_rescue_steps_name_serves_it() {
        let builder = rescued_charge(&[], &[&PAYMENTS])
            .provider(payments("backup").step(StepName::new("charge_rescue")));
        assert!(builder.build().is_ok());
    }
}
