//! Fakes: what a test gives in place of a service provider's function, to supply the steps it
//! serves with instances of its own.

use std::future;

use super::{AnyProvided, Provided, ProviderEntry, ProviderName, Scope, ServiceType};
use crate::error::Error;
use crate::journey::JourneyId;
use crate::policy::{Answers, HookCall, Needs};
use crate::step::{StepAttempt, StepName};

/// How a fake makes an instance, whatever the service's type.
type Make = dyn Fn() -> Result<AnyProvided, Error> + Send + Sync;

/// A fake's function, with the type of the service it supplies.
pub(crate) struct Fake {
    service_type: ServiceType,
    make: Box<Make>,
}

impl Fake {
    pub(crate) fn new<T: ?Sized + Send + Sync + 'static>(
        make: impl Fn() -> Result<Provided<T>, Error> + Send + Sync + 'static,
    ) -> Self {
        Self {
            service_type: ServiceType::of::<T>(),
            make: Box::new(move || erased(&make)),
        }
    }
}

fn erased<T: ?Sized + Send + Sync + 'static>(
    make: &impl Fn() -> Result<Provided<T>, Error>,
) -> Result<AnyProvided, Error> {
    make().map(Provided::erased)
}

/// A service provider whose function a fake replaces: it keeps the provider's name, service,
/// scope, attachment and needs, so the executor resolves what the provider requests and keeps
/// what it makes as it would, and only calls the fake instead.
#[derive(derive_more::Debug)]
pub(crate) struct Faked {
    name: ProviderName,
    key: &'static str,
    service_type: ServiceType,
    scope: Scope,
    steps: Vec<StepName>,
    needs: Needs,
    #[debug(skip)]
    make: Box<Make>,
}

impl Faked {
    /// `provider`, with `fake` in place of its function.
    ///
    /// # Panics
    ///
    /// When the fake supplies another type of service than the provider.
    #[track_caller]
    #[expect(
        clippy::panic,
        reason = "a fake of the wrong type is a mistake in the test, found before its journey"
    )]
    pub(crate) fn new<W, M>(provider: &dyn ProviderEntry<W, M>, fake: Fake) -> Self {
        let name = provider.name();
        let service_type = provider.service_type();
        if fake.service_type != service_type {
            panic!(
                "the fake of the service provider {name} supplies {:?}, but the provider \
                 supplies {service_type:?}: name its type, as in `fake_provider::<{service_type:?}>`",
                fake.service_type
            );
        }
        Self {
            name,
            key: provider.key(),
            service_type,
            scope: provider.scope(),
            steps: provider.steps().to_vec(),
            needs: provider.needs().clone(),
            make: fake.make,
        }
    }
}

impl<W, M> ProviderEntry<W, M> for Faked {
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
        self.scope
    }

    fn steps(&self) -> &[StepName] {
        &self.steps
    }

    fn needs(&self) -> &Needs {
        &self.needs
    }

    fn provide<'a>(
        &'a self,
        _workflow: &'a W,
        _journey_id: &'a JourneyId,
        _attempt: StepAttempt,
        _answers: Answers<'a>,
    ) -> HookCall<'a, AnyProvided> {
        Box::pin(future::ready((self.make)()))
    }
}

#[cfg(test)]
mod tests;
