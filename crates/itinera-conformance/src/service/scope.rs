//! The scopes of a scripted provider: what it may request, and how it is described in each mode.

use itinera::error::Error;
use itinera::mode::Asynchronous;
use itinera::policy::HookNeeds;
use itinera::service::{
    AsyncProvide, PerAttempt, PerJourney, Provided, Provider, ProviderScope, Service,
    ServiceProviderDescriptor,
};

use super::{Got, Supplied};
use crate::declaration::ScriptedWorkflow;
use crate::policy::{Request, declare_for_any, receive_for_any};
use crate::witness::Received;

/// A scope of a scripted provider, as its script declares what it requests and reads what it
/// received, and as it is described in either mode.
pub(crate) trait Scope: ProviderScope {
    /// Declares one request, or names it when a provider of this scope may not make it.
    fn declare(
        needs: HookNeeds<Provider<Self>>,
        request: &Request,
    ) -> Result<HookNeeds<Provider<Self>>, String>;

    /// Takes what the provider received for one request.
    fn receive<M>(
        got: &mut Got<'_, Self, M>,
        request: &Request,
        received: &mut Received,
    ) -> Result<(), Error>;

    /// Describes a provider of a synchronous workflow, with its name, its service and its
    /// function.
    fn synchronous<T: Supplied + ?Sized>(
        name: &'static str,
        service: &Service<T>,
        provide: impl for<'a> Fn(&'a ScriptedWorkflow, Got<'a, Self>) -> Result<Provided<T>, Error>
        + Send
        + Sync
        + 'static,
    ) -> ServiceProviderDescriptor<ScriptedWorkflow, Self>;

    /// Describes a provider of an asynchronous workflow, with its name, its service and its
    /// function.
    fn asynchronous<T: Supplied + ?Sized>(
        name: &'static str,
        service: &Service<T>,
        provide: impl for<'a> AsyncProvide<'a, ScriptedWorkflow, Self, T>,
    ) -> ServiceProviderDescriptor<ScriptedWorkflow, Self, Asynchronous>;
}

impl Scope for PerJourney {
    fn declare(
        needs: HookNeeds<Provider<Self>>,
        request: &Request,
    ) -> Result<HookNeeds<Provider<Self>>, String> {
        declare_for_any(needs, request)
    }

    fn receive<M>(
        got: &mut Got<'_, Self, M>,
        request: &Request,
        received: &mut Received,
    ) -> Result<(), Error> {
        receive_for_any(got, request, received)
    }

    fn synchronous<T: Supplied + ?Sized>(
        name: &'static str,
        service: &Service<T>,
        provide: impl for<'a> Fn(&'a ScriptedWorkflow, Got<'a, Self>) -> Result<Provided<T>, Error>
        + Send
        + Sync
        + 'static,
    ) -> ServiceProviderDescriptor<ScriptedWorkflow, Self> {
        ServiceProviderDescriptor::new(name, service, provide)
    }

    fn asynchronous<T: Supplied + ?Sized>(
        name: &'static str,
        service: &Service<T>,
        provide: impl for<'a> AsyncProvide<'a, ScriptedWorkflow, Self, T>,
    ) -> ServiceProviderDescriptor<ScriptedWorkflow, Self, Asynchronous> {
        ServiceProviderDescriptor::new_async(name, service, provide)
    }
}

impl Scope for PerAttempt {
    fn declare(
        needs: HookNeeds<Provider<Self>>,
        request: &Request,
    ) -> Result<HookNeeds<Provider<Self>>, String> {
        match request {
            Request::StepName | Request::Attempt => Ok(needs),
            _ => declare_for_any(needs, request),
        }
    }

    fn receive<M>(
        got: &mut Got<'_, Self, M>,
        request: &Request,
        received: &mut Received,
    ) -> Result<(), Error> {
        match request {
            Request::StepName => received.step_name = Some(got.step_name().to_string()),
            Request::Attempt => received.attempt = Some(got.attempt().into()),
            _ => return receive_for_any(got, request, received),
        }
        Ok(())
    }

    fn synchronous<T: Supplied + ?Sized>(
        name: &'static str,
        service: &Service<T>,
        provide: impl for<'a> Fn(&'a ScriptedWorkflow, Got<'a, Self>) -> Result<Provided<T>, Error>
        + Send
        + Sync
        + 'static,
    ) -> ServiceProviderDescriptor<ScriptedWorkflow, Self> {
        ServiceProviderDescriptor::per_attempt(name, service, provide)
    }

    fn asynchronous<T: Supplied + ?Sized>(
        name: &'static str,
        service: &Service<T>,
        provide: impl for<'a> AsyncProvide<'a, ScriptedWorkflow, Self, T>,
    ) -> ServiceProviderDescriptor<ScriptedWorkflow, Self, Asynchronous> {
        ServiceProviderDescriptor::per_attempt_async(name, service, provide)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    /// Whether a provider of this scope may make the request.
    type Allows = fn(&Request) -> bool;

    fn allows<S: Scope>(request: &Request) -> bool {
        S::declare(HookNeeds::new(), request).is_ok()
    }

    #[rstest]
    #[case::the_step_name_for_each_attempt(Request::StepName, allows::<PerAttempt>, true)]
    #[case::the_attempt_for_each_attempt(Request::Attempt, allows::<PerAttempt>, true)]
    #[case::the_step_name_once_per_journey(Request::StepName, allows::<PerJourney>, false)]
    #[case::the_attempt_once_per_journey(Request::Attempt, allows::<PerJourney>, false)]
    #[case::the_journey_id_once_per_journey(Request::JourneyId, allows::<PerJourney>, true)]
    fn only_a_provider_called_for_each_attempt_requests_the_step_name_and_the_attempt(
        #[case] request: Request,
        #[case] scope: Allows,
        #[case] allowed: bool,
    ) {
        assert_eq!(scope(&request), allowed);
    }
}
