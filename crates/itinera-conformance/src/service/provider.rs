//! A scripted service provider: what it requests, emits and makes on every call, as the scenario
//! says.

use std::future::Future;
use std::num::NonZeroU32;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use futures::executor::block_on;
use itinera::error::Error;
use itinera::mode::{Asynchronous, Synchronous};
use itinera::policy::{HookNeeds, Provider, Requested};
use itinera::service::{AsyncProvide, Provided};
use serde_json::Value;

use super::{Got, Made, Scope, Supplied};
use crate::declaration::{ScriptedWorkflow, leaked};
use crate::model::{self, Implementation, ModelError, ProviderAction, ProviderFailure};
use crate::policy::{Emits, Request};
use crate::value::Typed;
use crate::witness::{Provision, Received, Witness};

/// One of the scenario's service providers, made anew for each journey, which records each of
/// its calls in the witness.
#[derive(Debug)]
pub(crate) struct ScriptedProvider {
    name: &'static str,
    requests: Vec<Request>,
    labelled_by: Option<String>,
    failure: Option<ProviderFailure>,
    actions: Vec<ProviderAction>,
    fails_when_used: Option<String>,
    implementation: Implementation,
    /// The data it supplies with each instance.
    data: Option<Value>,
    /// How many instances it made in the journey.
    made: AtomicU32,
    witness: Witness,
}

impl ScriptedProvider {
    pub(crate) fn of(provider: &model::Provider, witness: &Witness) -> Self {
        Self {
            name: leaked(&provider.name),
            requests: provider.requests.iter().filter_map(Request::of).collect(),
            labelled_by: provider.labelled_by.clone(),
            failure: provider.failure.clone(),
            actions: provider.actions.clone(),
            fails_when_used: provider.fails_when_used.clone(),
            implementation: provider.implementation,
            data: provider.data.clone(),
            made: AtomicU32::new(0),
            witness: witness.clone(),
        }
    }

    pub(crate) fn name(&self) -> &'static str {
        self.name
    }

    /// What it needs, as a provider of scope `S`: its requests, and a reporter, which it takes
    /// whether it emits or not.
    pub(crate) fn needs<S: Scope>(&self) -> Result<HookNeeds<Provider<S>>, ModelError> {
        self.requests
            .iter()
            .try_fold(HookNeeds::new().reporter(), S::declare)
            .map_err(|request| self.not_allowed(request))
    }

    /// A request it cannot make, as a provider of its scope.
    fn not_allowed(&self, request: String) -> ModelError {
        ModelError::ProviderRequestNotAllowed(self.name.to_owned(), request)
    }

    /// Takes what it requested, emits what its script says, then makes an instance of its
    /// implementation, with its data, unless it fails.
    async fn provide<'a, S: Scope, M, R: Emits, T: Supplied + ?Sized>(
        &self,
        mut got: Got<'a, S, M>,
        reporter: fn(&mut Got<'a, S, M>) -> Result<R, Error>,
    ) -> Result<Provided<T>, Error> {
        let mut received = Received::default();
        self.requests
            .iter()
            .try_for_each(|request| S::receive(&mut got, request, &mut received))?;
        self.witness.provided(Provision {
            provider: self.name.to_owned(),
            received: received.clone(),
        });
        let mut reporter = reporter(&mut got)?;
        for action in &self.actions {
            act(&mut reporter, action).await?;
        }
        if let Some(failure) = self
            .failure
            .as_ref()
            .filter(|failure| fails_on(failure, &received))
        {
            return Err(Error::msg(failure.message.clone()));
        }
        let number = NonZeroU32::MIN.saturating_add(self.made.fetch_add(1, Ordering::SeqCst));
        let made = Made {
            label: self.label(&received, number)?,
            number,
            failure: self.fails_when_used.clone(),
        };
        Ok(T::supplied(made, self.implementation, self.data.clone()))
    }

    /// The label of the instance of this number: the string received for the request it labels
    /// its instances with, or its name and the number.
    fn label(&self, received: &Received, number: NonZeroU32) -> Result<String, Error> {
        match &self.labelled_by {
            Some(key) => received_string(received, key)
                .map(str::to_owned)
                .ok_or_else(|| self.unlabelled(key)),
            None => Ok(format!("{} {number}", self.name)),
        }
    }

    /// The error of a call that received no string to label its instance with.
    fn unlabelled(&self, key: &str) -> Error {
        Error::msg(format!("{} received no string for \"{key}\"", self.name))
    }
}

/// Does one thing the provider's script says, through its reporter.
async fn act<R: Emits>(reporter: &mut R, action: &ProviderAction) -> Result<(), Error> {
    match action {
        ProviderAction::Emit(level, message) => reporter.emit(*level, message.clone()).await?,
        ProviderAction::SetGlobalMetadata { key, value } => {
            reporter
                .set_global_metadata(key.clone(), Typed::of(value)?)
                .await?;
        }
    }
    Ok(())
}

/// Whether the provider fails on a call that received this.
fn fails_on(failure: &ProviderFailure, received: &Received) -> bool {
    failure
        .when
        .as_ref()
        .is_none_or(|(key, value)| received_this(received, key, value))
}

/// Whether the call received this string for this request of data from the workflow.
fn received_this(received: &Received, key: &str, value: &str) -> bool {
    received_string(received, key) == Some(value)
}

/// The string the call received for this request of data from the workflow, if it received one.
fn received_string<'r>(received: &'r Received, key: &str) -> Option<&'r str> {
    received
        .workflow_data
        .get(key)
        .and_then(Option::as_ref)
        .and_then(Value::as_str)
}

/// Calls a provider of a synchronous workflow, whose instance is ready at once.
pub(crate) fn provided_now<S: Scope, T: Supplied + ?Sized>(
    provider: &ScriptedProvider,
    got: Got<'_, S>,
) -> Result<Provided<T>, Error> {
    block_on(provider.provide(got, Requested::<_, _, Synchronous>::reporter))
}

/// A scripted provider of an asynchronous workflow, whose calls are awaited.
#[derive(Debug, derive_more::From)]
pub(crate) struct Awaiting {
    provider: Arc<ScriptedProvider>,
}

impl<'a, S: Scope, T: Supplied + ?Sized> AsyncProvide<'a, ScriptedWorkflow, S, T> for Awaiting {
    type Future = Pin<Box<dyn Future<Output = Result<Provided<T>, Error>> + Send + 'a>>;

    fn provide(&self, _: &'a ScriptedWorkflow, got: Got<'a, S, Asynchronous>) -> Self::Future {
        Box::pin(awaited(Arc::clone(&self.provider), got))
    }
}

async fn awaited<S: Scope, T: Supplied + ?Sized>(
    provider: Arc<ScriptedProvider>,
    got: Got<'_, S, Asynchronous>,
) -> Result<Provided<T>, Error> {
    provider
        .provide(got, Requested::<_, _, Asynchronous>::reporter)
        .await
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use serde_json::json;

    use super::*;
    use crate::executor::Executor;
    use crate::journey::fixtures::{attempt, lines, ran, timed_out};
    use crate::model::{Attempt, Model, Scope, ServiceType};
    use crate::trace::Line;

    /// The charge step requests the payments service, made for each attempt, and times out
    /// once before it succeeds.
    fn provided_for_each_attempt(model: &mut Model) {
        let charge = model.step_mut("charge").unwrap();
        charge
            .services
            .push(("payments".to_owned(), ServiceType::Labeller));
        charge.retries = 1;
        charge.attempts = Some(vec![attempt(timed_out()), Attempt::success()]);
        model
            .workflow_mut()
            .unwrap()
            .providers
            .push(model::Provider::new(
                "gateway".to_owned(),
                "payments".to_owned(),
                ServiceType::Labeller,
                Scope::Attempt,
            ));
    }

    fn is_provided(line: &Line) -> bool {
        line.event == "service_provided"
            && line.has_cell("key", "payments")
            && line.has_cell("provider", "gateway")
    }

    #[test]
    fn both_executors_call_a_scripted_provider_to_the_same_events() {
        let local = lines(&ran(
            Executor::Local,
            &["charge"],
            provided_for_each_attempt,
        ));
        let asynchronous = lines(&ran(
            Executor::AsyncLocal,
            &["charge"],
            provided_for_each_attempt,
        ));
        assert_eq!(local.iter().filter(|line| is_provided(line)).count(), 2);
        assert_eq!(local, asynchronous);
    }

    fn received(gateway: &str) -> Received {
        Received {
            workflow_data: [("gateway".to_owned(), Some(json!(gateway)))].into(),
            ..Received::default()
        }
    }

    /// The provider fails only when it receives this string for "gateway".
    fn on(value: &str) -> Option<(String, String)> {
        Some(("gateway".to_owned(), value.to_owned()))
    }

    #[rstest]
    #[case::on_every_call(None, true)]
    #[case::when_it_receives_the_string(on("backup"), true)]
    #[case::when_it_receives_another_string(on("main"), false)]
    fn a_provider_that_fails_when_it_receives_a_string_fails_only_on_a_call_that_received_it(
        #[case] when: Option<(String, String)>,
        #[case] fails: bool,
    ) {
        let failure = ProviderFailure {
            message: "no backup".to_owned(),
            when,
        };
        assert_eq!(fails_on(&failure, &received("backup")), fails);
    }
}
