//! Supplying the services a step requests: the instances of journey scope already made, or new
//! ones from the providers that serve the step, once what each provider requests is resolved.

use super::attempt::could_not_build;
use super::end::End;
use super::inputs::Sources;
use super::request::Requesting;
use super::{Delivery, Journey};
use crate::error::Error;
use crate::event::{EventBody, HookSource, Supplier};
use crate::mode::Mode;
use crate::policy::Answers;
use crate::service::{Instance, ProviderEntry, Scope};
use crate::step::{Reporting, StepAttempt, StepDescriptor};
use crate::workflow::WorkflowDescriptor;

mod kept;

pub(super) use kept::Kept;

/// The services supplied to one attempt of a step, in the order it declares them.
#[derive(Default)]
pub(super) struct Provided<'k> {
    services: Vec<(&'static str, Supplied<'k>)>,
}

/// One service supplied to an attempt: kept for the journey, or made for this attempt only.
enum Supplied<'k> {
    Kept(&'k Instance),
    Made(Instance),
}

impl Provided<'_> {
    /// The services, by key, as the step borrows them for its attempt.
    pub(super) fn lent(&self) -> Vec<(&'static str, &Instance)> {
        self.services.iter().map(lent).collect()
    }
}

impl<D: Delivery> Journey<D> {
    /// Supplies the services the step requests for one attempt, in the order it declares them.
    pub(super) async fn provide<'k, W, M: Mode>(
        &mut self,
        descriptor: &WorkflowDescriptor<W, M>,
        sources: &Sources<'_, W>,
        kept: &'k Kept,
        step: &StepDescriptor<W, M>,
        attempt: &StepAttempt,
    ) -> Result<Provided<'k>, End> {
        let name = step.name();
        let mut provided = Provided::default();
        for need in step.needs().services() {
            let key = need.key();
            let Some(provider) = descriptor.provider(name, key) else {
                return Err(could_not_build(name, None, unserved(key)));
            };
            let supplied = match provider.scope() {
                Scope::Attempt => Supplied::Made(self.call(sources, provider, attempt).await?),
                Scope::Journey => {
                    let Some(slot) = kept.slot(provider.name()) else {
                        return Err(could_not_build(name, None, unserved(key)));
                    };
                    match slot.get() {
                        Some(instance) => Supplied::Kept(instance),
                        None => {
                            let made = self.call(sources, provider, attempt).await?;
                            Supplied::Kept(slot.get_or_init(|| made))
                        }
                    }
                }
            };
            provided.services.push((key, supplied));
        }
        Ok(provided)
    }

    /// Calls a provider for an attempt of a step it serves, once what it requests is resolved,
    /// and reports the instance it made, or its failure.
    async fn call<W, M>(
        &mut self,
        sources: &Sources<'_, W>,
        provider: &dyn ProviderEntry<W, M>,
        attempt: &StepAttempt,
    ) -> Result<Instance, End> {
        let name = provider.name();
        let source = HookSource::Provider {
            provider: name,
            step: attempt.clone(),
        };
        let requesting = Requesting::hook(&source);
        let needs = provider.needs();
        let mut answers = Answers::default();
        for request in needs.requests() {
            self.answer(&mut answers, &requesting, request, sources.data_bag, None)
                .await?;
        }
        if needs.wants_reporter() {
            answers.reporting = Some(Reporting::new(self, source));
        }
        let provided = provider
            .provide(
                sources.workflow,
                sources.journey_id,
                attempt.clone(),
                answers,
            )
            .await;
        if let Some(aborted) = self.interrupted.take() {
            return Err(End::Aborted(aborted));
        }
        let key = provider.key().to_owned();
        match provided {
            Ok(made) => {
                let (instance, implementation, data) = made.into_parts();
                self.emit(EventBody::ServiceProvided {
                    step: attempt.clone(),
                    key,
                    provider: name,
                    implementation,
                    data,
                })
                .await?;
                Ok(instance)
            }
            Err(error) => {
                self.emit(EventBody::ServiceProviderFailed {
                    step: attempt.clone(),
                    key,
                    provider: name,
                })
                .await?;
                let supplier = Supplier::Provider { provider: name };
                Err(could_not_build(attempt.step, Some(supplier), error))
            }
        }
    }
}

fn lent<'p>((key, supplied): &'p (&'static str, Supplied<'_>)) -> (&'static str, &'p Instance) {
    match supplied {
        Supplied::Kept(instance) => (key, instance),
        Supplied::Made(instance) => (key, instance),
    }
}

/// The error for a service no provider serves, which admission rules out.
fn unserved(key: &str) -> Error {
    Error::msg(format!("no service provider serves the service \"{key}\""))
}

#[cfg(test)]
mod tests;
