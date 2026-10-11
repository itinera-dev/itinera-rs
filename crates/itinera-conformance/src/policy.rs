//! The scenario's scripted policies: what each hook requests, does and returns, as the scenario
//! says, in the mode of the executor that runs them.

mod hooks;
mod needs;
mod reporter;
mod request;
mod requested;
mod returning;
mod script;
mod scripting;

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use itinera::error::Error;
use itinera::journey::Contributor;
use itinera::policy::Requested;

pub(crate) use needs::requiring_from_workflow;
pub(crate) use script::Scripts;

use reporter::Emits;
use script::Action;
use scripting::Scripting;

use crate::declaration::{ScriptedWorkflow, leaked};
use crate::model::{self, ModelError};
use crate::role::Roles;
use crate::step::contribute;
use crate::witness::{Call, Received, Witness};

/// An instance of one of the scenario's policies, of either kind, built for an attempt or a
/// journey; it does what the scripts of its hooks say.
#[derive(Debug)]
pub(crate) struct ScriptedPolicy {
    name: &'static str,
    scripts: Arc<Scripts>,
    /// The counter its hooks add their calls to, from 0 in each instance.
    count: AtomicI64,
    witness: Witness,
}

impl ScriptedPolicy {
    fn new(name: &'static str, scripts: Arc<Scripts>, witness: Witness) -> Self {
        Self {
            name,
            scripts,
            count: AtomicI64::new(0),
            witness,
        }
    }

    /// Takes what the hook requested, then does what its script says and returns what it says.
    async fn run<'a, H: Scripting, M, R: Emits>(
        &self,
        mut got: Requested<'a, ScriptedWorkflow, H, M>,
        reporter: fn(&mut Requested<'a, ScriptedWorkflow, H, M>) -> Result<R, Error>,
    ) -> Result<H::Returns, Error> {
        let script = H::script(&self.scripts).ok_or_else(unscripted::<H>)?;
        let mut received = Received::default();
        script
            .requests
            .iter()
            .try_for_each(|request| H::receive(&mut got, request, &mut received))?;
        self.witness.called(Call {
            policy: self.name.to_owned(),
            hook: H::HOOK,
            received,
        });
        let roles = got.role::<dyn Roles>();
        let mut contributor = got.contributor()?;
        let mut reporter = reporter(&mut got)?;
        for action in &script.actions {
            self.perform(action, roles, &mut contributor, &mut reporter)
                .await?;
        }
        script.returns.clone().map_err(Error::msg)
    }

    async fn perform<R: Emits>(
        &self,
        action: &Action,
        roles: &dyn Roles,
        contributor: &mut Contributor<'_>,
        reporter: &mut R,
    ) -> Result<(), Error> {
        match action {
            Action::Contribute(key, value) => contribute(contributor, key, value),
            Action::CountCalls(key) => {
                let count = self.count.fetch_add(1, Ordering::SeqCst) + 1;
                contributor.contribute(key, count);
            }
            Action::Emit(level, message) => reporter.emit(*level, message.clone()).await?,
            Action::CallRole { role, operation } => roles.call(role, operation)?,
        }
        Ok(())
    }
}

/// The error of a hook called without a script. Only a hook the policy does not define has
/// none, and such a hook is never declared.
fn unscripted<H: Scripting>() -> Error {
    Error::msg(format!("the hook {} has no script", H::HOOK))
}

/// How one of the scenario's policies is built: what its hooks do, and whether building it
/// fails, with this message.
#[derive(Debug)]
pub(crate) struct Building {
    name: &'static str,
    scripts: Arc<Scripts>,
    failure: Option<String>,
    witness: Witness,
}

impl Building {
    pub(crate) fn of(
        name: &str,
        policy: &model::Policy,
        witness: &Witness,
    ) -> Result<Self, ModelError> {
        Ok(Self {
            name: leaked(name),
            scripts: Arc::new(Scripts::of(name, policy)?),
            failure: policy.construction_failure.clone().map(failure_message),
            witness: witness.clone(),
        })
    }

    pub(crate) fn name(&self) -> &'static str {
        self.name
    }

    pub(crate) fn scripts(&self) -> Arc<Scripts> {
        Arc::clone(&self.scripts)
    }

    /// A new instance of the policy, unless building it fails.
    pub(crate) fn build(&self) -> Result<ScriptedPolicy, Error> {
        match &self.failure {
            Some(message) => Err(Error::msg(message.clone())),
            None => Ok(ScriptedPolicy::new(
                self.name,
                self.scripts(),
                self.witness.clone(),
            )),
        }
    }
}

fn failure_message(message: Option<String>) -> String {
    message.unwrap_or_else(|| "the scripted policy cannot be built".to_owned())
}
