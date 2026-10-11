//! The scenario's service providers, declared with their scripts.

use itinera::service::{PerAttempt, PerJourney, Service, ServiceProviderDescriptor};
use itinera::workflow::WorkflowBuilder;

use super::{Declares, ScriptedWorkflow, leaked, step_name};
use crate::model::{ModelError, Provider, Scope, ServiceType};
use crate::service::{self, Labeller, Meter, ScriptedProvider, Supplied};
use crate::witness::Witness;

/// The workflow's builder, with this provider too, of its scope and its type of service.
pub(super) fn with_provider<M: Declares>(
    builder: WorkflowBuilder<ScriptedWorkflow, M>,
    provider: &Provider,
    witness: &Witness,
) -> Result<WorkflowBuilder<ScriptedWorkflow, M>, ModelError> {
    Ok(match (provider.scope, provider.service_type) {
        (Scope::Journey, ServiceType::Labeller) => {
            builder.provider(described::<M, PerJourney, dyn Labeller>(provider, witness)?)
        }
        (Scope::Journey, ServiceType::Meter) => {
            builder.provider(described::<M, PerJourney, dyn Meter>(provider, witness)?)
        }
        (Scope::Attempt, ServiceType::Labeller) => {
            builder.provider(described::<M, PerAttempt, dyn Labeller>(provider, witness)?)
        }
        (Scope::Attempt, ServiceType::Meter) => {
            builder.provider(described::<M, PerAttempt, dyn Meter>(provider, witness)?)
        }
    })
}

/// The provider, of scope `S`, supplying its key as the service `T`, needing what its script
/// says and attached to the steps it serves only.
fn described<M: Declares, S: service::Scope, T: Supplied + ?Sized>(
    provider: &Provider,
    witness: &Witness,
) -> Result<ServiceProviderDescriptor<ScriptedWorkflow, S, M>, ModelError> {
    let scripted = ScriptedProvider::of(provider, witness);
    let needs = scripted.needs::<S>()?;
    let steps = provider
        .steps
        .iter()
        .map(String::as_str)
        .map(step_name)
        .collect::<Result<Vec<_>, _>>()?;
    let service = Service::<T>::new(leaked(&provider.key));
    let descriptor = M::provider(scripted, &service).needing(needs);
    Ok(steps
        .into_iter()
        .fold(descriptor, ServiceProviderDescriptor::step))
}

#[cfg(test)]
mod tests {
    use itinera::service::ProviderName;
    use rstest::rstest;

    use super::*;
    use crate::declaration::Admission;
    use crate::executor::Executor;
    use crate::journey::fixtures::{attempt, journeyed, timed_out};
    use crate::model::{Attempt, Model};

    /// The charge step requests the payments service of this type, served by the gateway
    /// provider of this scope, and times out once before it succeeds.
    fn served(model: &mut Model, scope: Scope, service_type: ServiceType) {
        let charge = model.step_mut("charge").unwrap();
        charge.services.push(("payments".to_owned(), service_type));
        charge.retries = 1;
        charge.attempts = Some(vec![attempt(timed_out()), Attempt::success()]);
        let mut provider = Provider::new(
            "gateway".to_owned(),
            "payments".to_owned(),
            service_type,
            scope,
        );
        provider.steps.push("charge".to_owned());
        model.workflow_mut().unwrap().providers.push(provider);
    }

    #[rstest]
    #[case::a_labeller_once_per_journey(Scope::Journey, ServiceType::Labeller, 1)]
    #[case::a_meter_once_per_journey(Scope::Journey, ServiceType::Meter, 1)]
    #[case::a_labeller_for_each_attempt(Scope::Attempt, ServiceType::Labeller, 2)]
    #[case::a_meter_for_each_attempt(Scope::Attempt, ServiceType::Meter, 2)]
    fn a_provider_of_each_scope_and_type_serves_the_step_it_is_attached_to(
        #[case] scope: Scope,
        #[case] service_type: ServiceType,
        #[case] calls: usize,
    ) {
        let world = journeyed(Executor::Local, &["charge"], |model| {
            served(model, scope, service_type)
        });

        let Some(Admission::Admitted(listing)) = &world.admission else {
            panic!("the workflow was not admitted: {:?}", world.admission);
        };
        let services = &listing[0].services;
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].key, "payments");
        assert_eq!(services[0].provider, ProviderName::from("gateway"));
        assert_eq!(world.witness.provisions().len(), calls);
    }
}
