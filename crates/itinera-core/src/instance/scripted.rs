//! Scripted runs for tests: a workflow instance run with what some attempts do written in a
//! script, instead of running their steps.

use std::marker::PhantomData;

use super::WorkflowInstance;
use crate::executor::{LocalExecutor, Refusal};
use crate::journey::ScriptedRun;
use crate::mode::Synchronous;
use crate::step::Script;
use slot::Slot;
use target::Target;

#[cfg(feature = "async")]
use crate::executor::AsyncLocalExecutor;
#[cfg(feature = "async")]
use crate::mode::Asynchronous;

mod attempt;
mod checks;
#[cfg(test)]
mod fixtures;
mod slot;
mod step;
mod target;

pub use attempt::{Acting, AttemptScripting, Named, Opened};
pub use step::StepScripting;

/// A workflow instance about to run with a script, for a test that checks the workflow's flow
/// without its steps' business rules. [`WorkflowInstance::scripted`] makes it.
///
/// The script says, for the steps it names, what each attempt does instead of running the step:
/// what it contributes, emits and sets as global metadata, and how it ends. Every step is still
/// built for real, with its inputs, its services and its constructor, so a missing input or a
/// failing constructor aborts the journey as it would otherwise; only the call to its `run` is
/// replaced. A step the script does not name runs for real. Everything else is the executor's:
/// hooks, policies, decisions, events and the result.
///
/// Steps are named by text, rescue steps by their own names, such as `charge_rescue`.
///
/// It is available with the `testing` feature, which an application enables under
/// `[dev-dependencies]`. `M` is the workflow's mode, which decides whether `run` is awaited.
///
/// # Examples
///
/// ```
/// use itinera::instance::WorkflowInstance;
/// use itinera::journey::StatusKind;
/// use itinera::step::{Outcome, StepDescriptor, step_name};
/// use itinera::workflow::WorkflowDescriptor;
///
/// # struct Charge<'a> {
/// #     contributor: itinera::journey::Contributor<'a>,
/// #     reporter: itinera::step::StepReporter<'a>,
/// # }
/// # impl itinera::step::Step for Charge<'_> {
/// #     fn run(mut self) -> Result<Outcome, itinera::error::Error> {
/// #         self.reporter.info("charging")?;
/// #         self.contributor.contribute("charged", 40_i64);
/// #         Ok(Outcome::success())
/// #     }
/// # }
/// # struct ChargeFactory;
/// # impl itinera::step::StepFactory for ChargeFactory {
/// #     type Step<'a> = Charge<'a>;
/// #     fn needs(&self) -> itinera::step::StepNeeds {
/// #         itinera::step::StepNeeds::new().contributor().reporter()
/// #     }
/// #     fn build<'a>(
/// #         &'a self,
/// #         got: &mut itinera::step::Resolved<'a>,
/// #     ) -> Result<Charge<'a>, itinera::error::Error> {
/// #         let contributor = got.contributor()?;
/// #         let reporter = got.reporter()?;
/// #         Ok(Charge { contributor, reporter })
/// #     }
/// # }
/// // ChargeFactory builds a charge step that declares a contributor and a reporter.
///
/// struct Orders;
///
/// let orders = WorkflowDescriptor::builder("orders")
///     .step(StepDescriptor::new(step_name!("charge"), ChargeFactory).retry_budget(1))
///     .step(StepDescriptor::new(step_name!("ship"), || Ok(Outcome::success())))
///     .build()?;
/// let run = orders
///     .instance(Orders)
///     .create()?
///     .scripted()
///     .step("charge")
///     .will_fail_retriably("declined")
///     .on_attempt(2)
///     .will_contribute("charged", 40_i64)
///     .will_succeed()
///     .run()?;
/// assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(derive_more::Debug)]
#[debug("Scripted({targets:?})")]
pub struct Scripted<I: WorkflowInstance, M = <I as WorkflowInstance>::Mode> {
    instance: I,
    /// The steps the script names, in the order it names them.
    targets: Vec<Target>,
    /// Whether every step the journey could reach must be scripted.
    all: bool,
    mode: PhantomData<fn() -> M>,
}

impl<I: WorkflowInstance, M> Scripted<I, M> {
    pub(super) fn new(instance: I) -> Self {
        Self {
            instance,
            targets: Vec::new(),
            all: false,
            mode: PhantomData,
        }
    }

    /// Starts the script of the step, or rescue step, with this name.
    ///
    /// # Panics
    ///
    /// When the workflow has no step or rescue step with this name, or the script already named
    /// it.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::journey::StatusKind;
    /// use itinera::step::{Outcome, RescueOutcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let charge = StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success()))
    ///     .rescue(|| Ok(RescueOutcome::success()));
    /// let orders = WorkflowDescriptor::builder("orders").step(charge).build()?;
    /// let run = orders
    ///     .instance(Orders)
    ///     .create()?
    ///     .scripted()
    ///     .step("charge")
    ///     .will_fail("expired-card")
    ///     .on_attempt(2)
    ///     .will_succeed()
    ///     .and_step("charge_rescue")
    ///     .will_succeed()
    ///     .run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[expect(
        clippy::panic,
        reason = "naming a step the workflow does not have is a mistake in a test's script"
    )]
    pub fn step(self, name: &str) -> AttemptScripting<I, M, Named> {
        let Some(target) = Target::named(self.instance.descriptor(), name) else {
            panic!(
                "the workflow {} has no step named {name}",
                self.instance.descriptor().name()
            );
        };
        if self.targets.iter().any(|scripted| scripted.is(target.name)) {
            panic!("{name} is scripted already");
        }
        AttemptScripting::new(StepScripting::new(self, target), Slot::Next)
    }

    /// Requires a script for every step of the workflow, so that none runs for real. `run`
    /// checks it before the journey starts. A rescue step is required when its step's script
    /// can fail on attempt 1, whether or not this is called.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::journey::StatusKind;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    ///     .build()?;
    /// let run = orders
    ///     .instance(Orders)
    ///     .create()?
    ///     .scripted()
    ///     .ensure_all_scripted()
    ///     .step("charge")
    ///     .will_succeed()
    ///     .run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn ensure_all_scripted(mut self) -> Self {
        self.all = true;
        self
    }

    /// Checks the script, and gives the instance with the engine's script.
    fn checked(self) -> (I, Script) {
        checks::check(self.instance.descriptor(), &self.targets, self.all);
        let script = self.targets.into_iter().map(Into::into).collect::<Vec<_>>();
        (self.instance, Script::from(script))
    }
}

impl<I: WorkflowInstance<Mode = Synchronous>> Scripted<I, Synchronous> {
    /// Runs the instance's journey with the script, as a [`LocalExecutor`] with its default
    /// dispatcher factory would, and gives back its result.
    ///
    /// # Panics
    ///
    /// Before the journey starts, listing everything missing, when a scripted step does not
    /// cover every attempt it can have, when a step's script can fail on attempt 1 and its rescue
    /// step has no script, or, after [`ensure_all_scripted`](Self::ensure_all_scripted), when a
    /// step has no script.
    ///
    /// # Errors
    ///
    /// The [`Refusal`] the executor gives when the journey cannot start.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::journey::StatusKind;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    ///     .build()?;
    /// let run = orders.instance(Orders).create()?.scripted().run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn run(self) -> Result<ScriptedRun, Refusal> {
        let (instance, script) = self.checked();
        let result = LocalExecutor::new().run_scripted(instance, script)?;
        Ok(ScriptedRun { result })
    }
}

#[cfg(feature = "async")]
impl<I: WorkflowInstance<Mode = Asynchronous>> Scripted<I, Asynchronous> {
    /// Runs the instance's journey with the script, as an [`AsyncLocalExecutor`] with its
    /// default dispatcher factory would, and gives back its result.
    ///
    /// # Panics
    ///
    /// Before the journey starts, listing everything missing, when a scripted step does not
    /// cover every attempt it can have, when a step's script can fail on attempt 1 and its rescue
    /// step has no script, or, after [`ensure_all_scripted`](Self::ensure_all_scripted), when a
    /// step has no script.
    ///
    /// # Errors
    ///
    /// The [`Refusal`] the executor gives when the journey cannot start.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::journey::StatusKind;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::async_builder("orders")
    ///     .step(StepDescriptor::new_async(step_name!("charge"), async || Ok(Outcome::success())))
    ///     .build()?;
    /// let scripted = orders.instance(Orders).create()?.scripted().step("charge").will_succeed();
    /// let run = futures::executor::block_on(scripted.run())?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub async fn run(self) -> Result<ScriptedRun, Refusal> {
        let (instance, script) = self.checked();
        let result = AsyncLocalExecutor::new()
            .run_scripted(instance, script)
            .await?;
        Ok(ScriptedRun { result })
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::{rescued, retried};
    #[cfg(feature = "async")]
    use crate::instance::WorkflowInstance;
    #[cfg(feature = "async")]
    use crate::journey::StatusKind;
    #[cfg(feature = "async")]
    use crate::step::{Outcome, StepDescriptor, StepName};
    #[cfg(feature = "async")]
    use crate::workflow::WorkflowDescriptor;
    #[cfg(feature = "async")]
    use crate::workflow::fixtures::Orders;

    #[test]
    #[should_panic(expected = "the workflow orders has no step named chrage")]
    fn a_step_the_workflow_does_not_have_cannot_be_scripted() {
        rescued().step("chrage");
    }

    #[test]
    #[should_panic(expected = "charge is scripted already")]
    fn a_step_cannot_be_scripted_twice() {
        retried(0).step("charge").will_succeed().and_step("charge");
    }

    #[cfg(feature = "async")]
    #[test]
    fn an_asynchronous_workflow_is_scripted_and_awaited() {
        let charge =
            StepDescriptor::new_async(StepName::new("charge"), async || Ok(Outcome::success()));
        let orders = WorkflowDescriptor::async_builder("orders")
            .step(charge)
            .build()
            .unwrap();
        let scripted = orders
            .instance(Orders)
            .create()
            .unwrap()
            .scripted()
            .step("charge")
            .will_fail("declined");

        let run = futures::executor::block_on(scripted.run()).unwrap();

        assert_eq!(run.result.status.kind(), StatusKind::Failed);
    }
}
