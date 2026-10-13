//! A workflow's service providers, whatever their scope, as the executor finds and calls them.

use std::fmt;

use super::ServiceProviderDescriptor;
use crate::journey::JourneyId;
use crate::policy::{Answers, HookCall, Needs, Requested};
use crate::service::{AnyProvided, ProviderName, ProviderScope, Scope, ServiceType};
use crate::step::{StepAttempt, StepName};

/// A service provider of a workflow, whatever its scope.
pub(crate) trait ProviderEntry<W, M>: Send + Sync + fmt::Debug {
    fn name(&self) -> ProviderName;

    /// The key of the service it supplies.
    fn key(&self) -> &'static str;

    fn service_type(&self) -> ServiceType;

    fn scope(&self) -> Scope;

    /// The steps it is attached to, none when it serves every step that requests its service.
    fn steps(&self) -> &[StepName];

    fn needs(&self) -> &Needs;

    /// Calls the provider for an attempt of a step it serves, once its requests are resolved.
    fn provide<'a>(
        &'a self,
        workflow: &'a W,
        journey_id: &'a JourneyId,
        attempt: StepAttempt,
        answers: Answers<'a>,
    ) -> HookCall<'a, AnyProvided>;
}

impl<W, S: ProviderScope, M> ProviderEntry<W, M> for ServiceProviderDescriptor<W, S, M>
where
    Self: Send + Sync,
{
    fn name(&self) -> ProviderName {
        self.name
    }

    fn key(&self) -> &'static str {
        self.key
    }

    fn service_type(&self) -> ServiceType {
        self.service_type
    }

    fn scope(&self) -> Scope {
        S::SCOPE
    }

    fn steps(&self) -> &[StepName] {
        &self.steps
    }

    fn needs(&self) -> &Needs {
        &self.needs
    }

    fn provide<'a>(
        &'a self,
        workflow: &'a W,
        journey_id: &'a JourneyId,
        attempt: StepAttempt,
        answers: Answers<'a>,
    ) -> HookCall<'a, AnyProvided> {
        (self.provide)(
            workflow,
            Requested::new(workflow, journey_id, attempt, answers),
        )
    }
}

/// The provider that serves a step or a rescue step for the service under `key`: the provider of
/// that service attached to its name, or else the one attached to no step.
pub(crate) fn serving<'p, W, M>(
    providers: &'p [Box<dyn ProviderEntry<W, M>>],
    step: StepName,
    key: &str,
) -> Option<&'p dyn ProviderEntry<W, M>> {
    attached(providers, step, key).or_else(|| unattached(providers, key))
}

/// The provider of the service under `key` attached to the step, if there is one.
fn attached<'p, W, M>(
    providers: &'p [Box<dyn ProviderEntry<W, M>>],
    step: StepName,
    key: &str,
) -> Option<&'p dyn ProviderEntry<W, M>> {
    providers
        .iter()
        .map(Box::as_ref)
        .find(|provider| is_attached_for(*provider, step, key))
}

/// The provider of the service under `key` attached to no step, if there is one.
fn unattached<'p, W, M>(
    providers: &'p [Box<dyn ProviderEntry<W, M>>],
    key: &str,
) -> Option<&'p dyn ProviderEntry<W, M>> {
    providers
        .iter()
        .map(Box::as_ref)
        .find(|provider| is_unattached_for(*provider, key))
}

fn is_attached_for<W, M>(provider: &dyn ProviderEntry<W, M>, step: StepName, key: &str) -> bool {
    provider.key() == key && provider.steps().contains(&step)
}

fn is_unattached_for<W, M>(provider: &dyn ProviderEntry<W, M>, key: &str) -> bool {
    provider.key() == key && provider.steps().is_empty()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::mode::Synchronous;
    use crate::service::fixtures::{Made, PAYMENTS, STOCK, per_attempt, per_journey};
    use crate::workflow::fixtures::Orders;

    /// A provider of payments attached to no step, declared before one attached to charge and one
    /// attached to the rescue step of charge, and a provider of stock attached to reserve only.
    fn providers() -> Vec<Box<dyn ProviderEntry<Orders, Synchronous>>> {
        let made = Made::default();
        vec![
            Box::new(per_journey("payments", &PAYMENTS, &made)),
            Box::new(per_attempt("charging", &PAYMENTS, &made).step(StepName::new("charge"))),
            Box::new(
                per_attempt("refreshing", &PAYMENTS, &made).step(StepName::new("charge_rescue")),
            ),
            Box::new(per_journey("stock", &STOCK, &made).step(StepName::new("reserve"))),
        ]
    }

    #[rstest]
    #[case::the_step_it_is_attached_to("charge", "payments", Some("charging"))]
    #[case::a_step_it_is_not_attached_to("ship", "payments", Some("payments"))]
    #[case::a_step_whose_only_provider_is_attached_to_another_step("ship", "stock", None)]
    #[case::the_rescue_step_it_is_attached_to("charge_rescue", "payments", Some("refreshing"))]
    #[case::a_rescue_step_whose_only_provider_is_attached_to_its_step(
        "reserve_rescue",
        "stock",
        None
    )]
    fn a_provider_attached_to_a_step_serves_it_and_the_one_attached_to_no_step_serves_the_rest(
        #[case] step: &'static str,
        #[case] key: &str,
        #[case] provider: Option<&'static str>,
    ) {
        let providers = providers();

        let serving = serving(&providers, StepName::new(step), key).map(ProviderEntry::name);

        assert_eq!(serving, provider.map(ProviderName::from));
    }
}
