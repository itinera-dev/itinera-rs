//! Scripting a step: choosing which of its attempts the next phrases describe.

use std::cmp::Ordering;
use std::num::NonZeroU32;

use super::attempt::{Named, Opened};
use super::slot::Slot;
use super::target::Target;
use super::{AttemptScripting, Scripted, WorkflowInstance};
use crate::executor::Refusal;
use crate::journey::ScriptedRun;
use crate::mode::Synchronous;
use crate::step::AttemptScript;

#[cfg(feature = "async")]
use crate::mode::Asynchronous;

/// The script of a step, after an attempt's ending: the next phrases choose more of its
/// attempts, name the next step, or run the journey.
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
///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())).retry_budget(2))
///     .step(StepDescriptor::new(step_name!("ship"), || Ok(Outcome::success())))
///     .build()?;
/// let run = orders
///     .instance(Orders)
///     .create()?
///     .scripted()
///     .step("charge")
///     .will_fail("declined")
///     .on_remaining_attempts()
///     .will_succeed()
///     .and_step("ship")
///     .will_succeed()
///     .run()?;
/// assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(derive_more::Debug)]
pub struct StepScripting<I: WorkflowInstance, M> {
    scripted: Scripted<I, M>,
    /// The step being scripted.
    target: Target,
}

impl<I: WorkflowInstance, M> StepScripting<I, M> {
    pub(super) fn new(scripted: Scripted<I, M>, target: Target) -> Self {
        Self { scripted, target }
    }

    /// Says what the attempt with this number does. A step's attempts are scripted from 1, in
    /// order, without gaps.
    ///
    /// # Panics
    ///
    /// When `attempt` is not the attempt after the last one scripted, when the step's remaining
    /// attempts are scripted already, or when the step cannot have that many attempts: one for
    /// a rescue step, two for a step with a rescue step, and otherwise one more than its retry
    /// budget.
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
    ///     .step(
    ///         StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success()))
    ///             .retry_budget(1),
    ///     )
    ///     .build()?;
    /// let run = orders
    ///     .instance(Orders)
    ///     .create()?
    ///     .scripted()
    ///     .step("charge")
    ///     .will_fail("declined")
    ///     .on_attempt(2)
    ///     .will_succeed()
    ///     .run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[expect(
        clippy::panic,
        reason = "an attempt out of order is a mistake in a test's script"
    )]
    pub fn on_attempt(self, attempt: u32) -> AttemptScripting<I, M, Opened> {
        let target = self.target();
        let next = target.script.numbered().saturating_add(1);
        if target.script.covers_the_rest() {
            panic!(
                "the remaining attempts of {} are scripted already",
                target.name
            );
        }
        let Some(number) = NonZeroU32::new(attempt) else {
            panic!("attempts are numbered from 1");
        };
        match attempt.cmp(&next) {
            Ordering::Less => panic!("attempt {attempt} of {} is scripted already", target.name),
            Ordering::Greater => {
                panic!(
                    "attempt {next} of {} is missing before attempt {attempt}",
                    target.name
                )
            }
            Ordering::Equal => {}
        }
        if number > target.most_attempts() {
            panic!(
                "{} has no attempt {attempt}; it can have at most {}",
                target.name,
                target.most_attempts()
            );
        }
        AttemptScripting::new(self, Slot::Next)
    }

    /// Says what every attempt of the step does.
    ///
    /// # Panics
    ///
    /// When some of its attempts are scripted already.
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
    ///     .step(
    ///         StepDescriptor::new(step_name!("ship"), || Ok(Outcome::success()))
    ///             .retry_budget(3),
    ///     )
    ///     .build()?;
    /// let run = orders
    ///     .instance(Orders)
    ///     .create()?
    ///     .scripted()
    ///     .step("charge")
    ///     .will_succeed()
    ///     .and_step("ship")
    ///     .on_every_attempt()
    ///     .will_fail("no-courier")
    ///     .run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Failed);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[expect(
        clippy::panic,
        reason = "scripting an attempt twice is a mistake in a test's script"
    )]
    pub fn on_every_attempt(self) -> AttemptScripting<I, M, Opened> {
        let target = self.target();
        if target.script.numbered() > 0 || target.script.covers_the_rest() {
            panic!(
                "some attempts of {} are scripted already; on_remaining_attempts() covers the rest",
                target.name
            );
        }
        AttemptScripting::new(self, Slot::Remaining)
    }

    /// Says what every attempt of the step after the last numbered one does.
    ///
    /// # Panics
    ///
    /// When the remaining attempts are scripted already, or the step cannot have an attempt after
    /// the last numbered one.
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
    ///     .step(
    ///         StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success()))
    ///             .retry_budget(3),
    ///     )
    ///     .build()?;
    /// let run = orders
    ///     .instance(Orders)
    ///     .create()?
    ///     .scripted()
    ///     .step("charge")
    ///     .will_fail("declined")
    ///     .on_remaining_attempts()
    ///     .will_fail("timeout")
    ///     .run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Failed);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[expect(
        clippy::panic,
        reason = "scripting an attempt the step cannot have is a mistake in a test's script"
    )]
    pub fn on_remaining_attempts(self) -> AttemptScripting<I, M, Opened> {
        let target = self.target();
        let numbered = target.script.numbered();
        if target.script.covers_the_rest() {
            panic!(
                "the remaining attempts of {} are scripted already",
                target.name
            );
        }
        if numbered >= target.most_attempts().get() {
            panic!("{} has no attempt after attempt {numbered}", target.name);
        }
        AttemptScripting::new(self, Slot::Remaining)
    }

    /// Starts the script of the next step, or rescue step, with this name.
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
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    ///     .step(StepDescriptor::new(step_name!("ship"), || Ok(Outcome::success())))
    ///     .build()?;
    /// let run = orders
    ///     .instance(Orders)
    ///     .create()?
    ///     .scripted()
    ///     .step("charge")
    ///     .will_succeed()
    ///     .and_step("ship")
    ///     .will_skip()
    ///     .run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn and_step(self, name: &str) -> AttemptScripting<I, M, Named> {
        self.finished().step(name)
    }

    /// Requires a script for every step of the workflow, as
    /// [`Scripted::ensure_all_scripted`] does.
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
    ///     .step("charge")
    ///     .will_succeed()
    ///     .ensure_all_scripted()
    ///     .run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn ensure_all_scripted(self) -> Self {
        Self::new(self.scripted.ensure_all_scripted(), self.target)
    }

    /// The step being scripted.
    pub(super) fn target(&self) -> &Target {
        &self.target
    }

    /// Adds what the attempts in `slot` do to the step's script.
    pub(super) fn scripted(mut self, slot: Slot, attempt: AttemptScript) -> Self {
        match slot {
            Slot::Next => self.target.script.push(attempt),
            Slot::Remaining => self.target.script.cover_the_rest(attempt),
        }
        self
    }

    /// The whole script so far, with the step's.
    fn finished(mut self) -> Scripted<I, M> {
        self.scripted.targets.push(self.target);
        self.scripted
    }
}

impl<I: WorkflowInstance<Mode = Synchronous>> StepScripting<I, Synchronous> {
    /// Runs the instance's journey with the script, as [`Scripted::run`] does.
    ///
    /// # Panics
    ///
    /// Before the journey starts, listing everything missing, when a scripted step does not
    /// cover every attempt it can have, when a step's script can fail on attempt 1 and its rescue
    /// step has no script, or, after `ensure_all_scripted`, when a step has no script.
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
    /// let run = orders
    ///     .instance(Orders)
    ///     .create()?
    ///     .scripted()
    ///     .step("charge")
    ///     .will_fail("declined")
    ///     .run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Failed);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn run(self) -> Result<ScriptedRun, Refusal> {
        self.finished().run()
    }
}

#[cfg(feature = "async")]
impl<I: WorkflowInstance<Mode = Asynchronous>> StepScripting<I, Asynchronous> {
    /// Runs the instance's journey with the script, as [`Scripted::run`] does.
    ///
    /// # Panics
    ///
    /// Before the journey starts, listing everything missing, when a scripted step does not
    /// cover every attempt it can have, when a step's script can fail on attempt 1 and its rescue
    /// step has no script, or, after `ensure_all_scripted`, when a step has no script.
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
    /// let scripted = orders
    ///     .instance(Orders)
    ///     .create()?
    ///     .scripted()
    ///     .step("charge")
    ///     .will_fail("declined");
    /// let run = futures::executor::block_on(scripted.run())?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Failed);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub async fn run(self) -> Result<ScriptedRun, Refusal> {
        self.finished().run().await
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::{Orderly, OrderlyStep, retried};

    fn charge(scripted: Orderly) -> OrderlyStep {
        scripted.step("charge").will_fail("timeout")
    }

    #[test]
    #[should_panic(expected = "attempts are numbered from 1")]
    fn attempts_are_numbered_from_1() {
        charge(retried(3)).on_attempt(0);
    }

    #[test]
    #[should_panic(expected = "attempt 1 of charge is scripted already")]
    fn an_attempt_cannot_be_scripted_twice() {
        charge(retried(3)).on_attempt(1);
    }

    #[test]
    #[should_panic(expected = "attempt 2 of charge is missing before attempt 3")]
    fn attempts_are_scripted_without_gaps() {
        charge(retried(3)).on_attempt(3);
    }

    #[test]
    #[should_panic(expected = "charge has no attempt 2; it can have at most 1")]
    fn a_step_cannot_be_scripted_past_the_attempts_its_retry_budget_allows() {
        charge(retried(0)).on_attempt(2);
    }

    #[test]
    #[should_panic(expected = "the remaining attempts of charge are scripted already")]
    fn no_attempt_can_be_numbered_after_the_remaining_ones_are_scripted() {
        charge(retried(3))
            .on_remaining_attempts()
            .will_succeed()
            .on_attempt(2);
    }

    #[test]
    #[should_panic(
        expected = "some attempts of charge are scripted already; on_remaining_attempts() covers the rest"
    )]
    fn every_attempt_cannot_be_scripted_after_a_numbered_one() {
        charge(retried(3)).on_every_attempt();
    }

    #[test]
    #[should_panic(expected = "charge has no attempt after attempt 1")]
    fn the_remaining_attempts_cannot_be_scripted_when_none_remain() {
        charge(retried(0)).on_remaining_attempts();
    }
}
