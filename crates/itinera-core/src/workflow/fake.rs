//! The fakes a test gives in place of a workflow's service providers, for one instance.

use std::sync::Arc;

use super::WorkflowDescriptor;
use crate::mode::Mode;
use crate::service::{Fake, Faked, ProviderEntry, ProviderName};

impl<W, M: Mode> WorkflowDescriptor<W, M> {
    /// Puts `fake` in place of the function of the provider named `name`, for this descriptor
    /// alone, replacing any fake given before for that provider.
    ///
    /// # Panics
    ///
    /// When the workflow has no provider of that name, or the fake supplies another type of
    /// service than the provider.
    #[track_caller]
    #[expect(
        clippy::panic,
        reason = "faking a provider the workflow lacks is a mistake in the test, found before \
                  its journey"
    )]
    pub(crate) fn fake_provider(&mut self, name: ProviderName, fake: Fake) {
        let Some(provider) = self
            .declaration
            .providers
            .iter()
            .map(Box::as_ref)
            .find(|provider| named(*provider, name))
        else {
            panic!(
                "the workflow {} has no service provider named {name}",
                self.declaration.name
            );
        };
        let faked = Faked::new(provider, fake);
        self.fakes.insert(name, Arc::new(faked));
    }

    /// The provider, or the fake given in its place.
    pub(super) fn faked<'d>(
        &'d self,
        provider: &'d dyn ProviderEntry<W, M>,
    ) -> &'d dyn ProviderEntry<W, M> {
        match self.fakes.get(&provider.name()) {
            Some(fake) => fake.as_ref(),
            None => provider,
        }
    }
}

fn named<W, M>(provider: &dyn ProviderEntry<W, M>, name: ProviderName) -> bool {
    provider.name() == name
}

#[cfg(test)]
mod tests {
    use crate::engine::fixtures::travel;
    use crate::error::Error;
    use crate::service::Provided;
    use crate::service::fixtures::{
        Given, Labeller, Made, PAYMENTS, given, labelled, per_journey, requesting,
    };
    use crate::step::StepName;
    use crate::workflow::WorkflowDescriptor;
    use crate::workflow::fixtures::{Orders, orders};

    fn first() -> Result<Provided<dyn Labeller>, Error> {
        Ok(labelled("first fake", &Made::default()))
    }

    fn second() -> Result<Provided<dyn Labeller>, Error> {
        Ok(labelled("second fake", &Made::default()))
    }

    /// The orders workflow with one step, charge, which requests the payments service from a
    /// provider of journey scope and records the label of each instance in `charged`.
    fn charging(charged: &Given) -> WorkflowDescriptor<Orders> {
        orders()
            .step(requesting("charge", &[&PAYMENTS], &[], charged))
            .provider(per_journey("payments", &PAYMENTS, &Made::default()))
            .build()
            .unwrap()
    }

    #[test]
    #[should_panic(expected = "the workflow orders has no service provider named paymnets")]
    fn faking_a_provider_the_workflow_lacks_panics_naming_it() {
        charging(&Given::default())
            .instance(Orders)
            .fake_provider::<dyn Labeller>("paymnets", first);
    }

    #[test]
    fn a_fake_serves_only_the_instance_it_was_given_to() {
        let charged = Given::default();
        let workflow = charging(&charged);

        travel(
            workflow
                .instance(Orders)
                .fake_provider::<dyn Labeller>("payments", first)
                .create()
                .unwrap(),
        );
        travel(workflow.instance(Orders).create().unwrap());

        assert_eq!(given(&charged), ["first fake 1", "payments 1"]);
    }

    #[test]
    fn a_later_fake_of_a_provider_replaces_an_earlier_one() {
        let charged = Given::default();

        travel(
            charging(&charged)
                .instance(Orders)
                .fake_provider::<dyn Labeller>("payments", first)
                .fake_provider::<dyn Labeller>("payments", second)
                .create()
                .unwrap(),
        );

        assert_eq!(given(&charged), ["second fake 1"]);
    }

    #[test]
    fn a_fake_replaces_only_the_provider_it_names() {
        let (reserved, charged) = (Given::default(), Given::default());
        let shared =
            per_journey("shared", &PAYMENTS, &Made::default()).step(StepName::new("reserve"));
        let workflow = orders()
            .step(requesting("reserve", &[&PAYMENTS], &[], &reserved))
            .step(requesting("charge", &[&PAYMENTS], &[], &charged))
            .provider(per_journey("payments", &PAYMENTS, &Made::default()))
            .provider(shared)
            .build()
            .unwrap();

        travel(
            workflow
                .instance(Orders)
                .fake_provider::<dyn Labeller>("shared", first)
                .create()
                .unwrap(),
        );

        assert_eq!(given(&reserved), ["first fake 1"]);
        assert_eq!(given(&charged), ["payments 1"]);
    }
}
