//! The steps a script names, with what the workflow says about their attempts.

use std::num::NonZeroU32;

use crate::mode::Mode;
use crate::step::{Action, StepDescriptor, StepName, StepScript};
use crate::workflow::WorkflowDescriptor;

/// A step the script names, with its script so far. It converts into the step's name with its
/// script, as the engine looks them up.
#[derive(Debug, derive_more::Into)]
pub(super) struct Target {
    pub(super) name: StepName,
    #[into(skip)]
    pub(super) kind: Kind,
    /// Whether the step declares a contributor.
    #[into(skip)]
    contributes: bool,
    /// Whether the step declares a reporter.
    #[into(skip)]
    reports: bool,
    pub(super) script: StepScript,
}

/// What the workflow says about a scripted step's attempts.
#[derive(Clone, Copy, Debug)]
pub(super) enum Kind {
    /// A step of the workflow.
    Step {
        /// How many attempts its retry budget allows.
        attempts: NonZeroU32,
        /// Whether an abnormal termination is retried, within that budget.
        retry_on_abnormal_termination: bool,
        /// Its rescue step's name, if it has one.
        rescue: Option<StepName>,
    },
    /// A step's rescue step, attempted once each time it runs.
    Rescue,
}

impl Target {
    /// The step, or rescue step, named `name` in the workflow, if it has one.
    pub(super) fn named<W: Send + Sync + 'static, M: Mode>(
        descriptor: &WorkflowDescriptor<W, M>,
        name: &str,
    ) -> Option<Self> {
        descriptor
            .steps()
            .iter()
            .find_map(|step| target_in(step, name))
    }

    /// How many attempts the step can have: one for a rescue step, two for a step with a rescue
    /// step, the attempt before the rescue and the one after, and otherwise as many as its retry
    /// budget allows.
    pub(super) fn most_attempts(&self) -> NonZeroU32 {
        match self.kind {
            Kind::Rescue => NonZeroU32::MIN,
            Kind::Step {
                rescue: Some(_), ..
            } => NonZeroU32::MIN.saturating_add(1),
            Kind::Step { attempts, .. } => attempts,
        }
    }

    /// Checks that the step declares the handle the real step would do this through.
    ///
    /// # Panics
    ///
    /// When the step declares no contributor and the action contributes, or no reporter and the
    /// action emits or sets global metadata.
    #[expect(
        clippy::panic,
        reason = "scripting what the real step could not do is a mistake in a test's script"
    )]
    pub(super) fn check(&self, action: &Action) {
        let (declared, handle, doing) = match action {
            Action::Contribute { .. } => (self.contributes, "contributor", "contribute"),
            Action::Emit { .. } => (self.reports, "reporter", "emit"),
            Action::SetGlobalMetadata { .. } => (self.reports, "reporter", "set global metadata"),
        };
        if !declared {
            panic!("{} declares no {handle}, so it cannot {doing}", self.name);
        }
    }

    pub(super) fn is(&self, name: StepName) -> bool {
        self.name == name
    }

    pub(super) fn is_rescue(&self) -> bool {
        matches!(self.kind, Kind::Rescue)
    }

    fn new<W, M>(step: &StepDescriptor<W, M>, kind: Kind) -> Self {
        Self {
            name: step.name(),
            kind,
            contributes: step.needs().wants_contributor(),
            reports: step.needs().wants_reporter(),
            script: StepScript::default(),
        }
    }
}

/// The step, or its rescue step, if either is named `name`.
fn target_in<W, M>(step: &StepDescriptor<W, M>, name: &str) -> Option<Target> {
    if step.name().is(name) {
        Some(Target::new(
            step,
            Kind::Step {
                attempts: step.attempts_allowed(),
                retry_on_abnormal_termination: step.retries_abnormal_termination(),
                rescue: step.rescue_step().map(StepDescriptor::name),
            },
        ))
    } else {
        step.rescue_step()
            .filter(|rescue| is_named(rescue, name))
            .map(|rescue| Target::new(rescue, Kind::Rescue))
    }
}

fn is_named<W, M>(step: &StepDescriptor<W, M>, name: &str) -> bool {
    step.name().is(name)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::super::fixtures::{Charge, rescued, retried};
    use crate::event::Level;
    use crate::journey::StatusKind;

    fn contributes(charge: Charge) {
        charge.will_contribute("charged", 40_i64);
    }

    fn emits(charge: Charge) {
        charge.will_emit(Level::Info, "charging");
    }

    fn sets_global_metadata(charge: Charge) {
        charge.will_set_global_metadata("receipt", "R-1".to_owned());
    }

    #[rstest]
    #[case::a_contribution_without_a_contributor(contributes)]
    #[case::an_emit_without_a_reporter(emits)]
    #[case::global_metadata_without_a_reporter(sets_global_metadata)]
    #[should_panic(expected = "charge declares no ")]
    fn a_script_cannot_do_what_its_step_declares_no_handle_for(#[case] action: fn(Charge)) {
        action(retried(0).step("charge"));
    }

    #[test]
    fn a_rescue_step_is_scripted_by_its_own_name() {
        let run = rescued()
            .step("charge")
            .will_fail("expired-card")
            .on_attempt(2)
            .will_succeed()
            .and_step("charge_rescue")
            .will_fail("no-refresh")
            .run()
            .unwrap();

        assert_eq!(run.result.status.kind(), StatusKind::Failed);
    }

    #[test]
    #[should_panic(expected = "charge_rescue has no attempt 2; it can have at most 1")]
    fn a_rescue_step_has_one_attempt() {
        rescued().step("charge_rescue").will_succeed().on_attempt(2);
    }

    #[test]
    #[should_panic(expected = "charge has no attempt 3; it can have at most 2")]
    fn a_rescued_step_has_two_attempts() {
        rescued()
            .step("charge")
            .will_fail("declined")
            .on_attempt(2)
            .will_succeed()
            .on_attempt(3);
    }
}
