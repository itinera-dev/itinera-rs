use std::sync::Arc;

use rstest::rstest;

use crate::engine::fixtures::{
    CHARGE, Orders, Seen, broken, charge_succeeding, data_of, declined, kinds, orders, seen,
    succeed, travel_workflow,
};
use crate::error::Error;
use crate::journey::JourneyStatus;
use crate::policy::{
    HookNeeds, OnWorkflowFailure, OnWorkflowSuccess, Provides, Requested, WorkflowFailure,
    WorkflowPolicyDescriptor, WorkflowSuccess,
};
use crate::step::{Outcome, StepDescriptor};
use crate::workflow::WorkflowBuilder;

/// The workflow hooks, which record which of them was called.
struct Close {
    seen: Seen,
}

impl OnWorkflowSuccess<Orders> for Close {
    fn on_workflow_success(&self, _: Requested<'_, Orders, WorkflowSuccess>) -> Result<(), Error> {
        self.seen.lock().unwrap().push("success".to_string());
        Ok(())
    }
}

impl OnWorkflowFailure<Orders> for Close {
    fn on_workflow_failure(&self, _: Requested<'_, Orders, WorkflowFailure>) -> Result<(), Error> {
        self.seen.lock().unwrap().push("failure".to_string());
        Ok(())
    }
}

fn closing(seen: &Seen) -> WorkflowBuilder<Orders> {
    let close = Arc::clone(seen);
    orders().policy(
        WorkflowPolicyDescriptor::new("close", move || Close {
            seen: Arc::clone(&close),
        })
        .on_workflow_success()
        .on_workflow_failure(),
    )
}

#[rstest]
#[case::a_success(succeed, &["success"], "journey_succeeded")]
#[case::a_failure(declined, &["failure"], "journey_failed")]
fn the_workflow_hook_for_how_the_journey_ended_runs_before_it_is_reported(
    #[case] ends: fn() -> Result<Outcome, Error>,
    #[case] called: &[&str],
    #[case] last: &str,
) {
    let saw = Seen::default();
    let workflow = closing(&saw).step(StepDescriptor::new(CHARGE, ends));

    let (_, events) = travel_workflow(workflow);

    assert_eq!(seen(&saw), called);
    assert_eq!(kinds(&events)[events.len() - 2..], ["hook_called", last]);
}

/// What the workflow knows of its customers' balances, a role it provides to its policies.
trait Balances {
    fn balance(&self) -> i64;
}

impl Balances for Orders {
    fn balance(&self) -> i64 {
        42
    }
}

impl Provides<dyn Balances> for Orders {
    fn role(&self) -> &(dyn Balances + 'static) {
        self
    }
}

/// `on workflow success`, which contributes the balance the workflow's role gives.
struct Settle;

impl OnWorkflowSuccess<Orders> for Settle {
    fn needs() -> HookNeeds<WorkflowSuccess> {
        HookNeeds::new().contributor()
    }

    fn on_workflow_success(
        &self,
        mut got: Requested<'_, Orders, WorkflowSuccess>,
    ) -> Result<(), Error> {
        let balance = got.role::<dyn Balances>().balance();
        got.contributor()?.contribute("balance", balance);
        Ok(())
    }
}

#[test]
fn a_workflow_hooks_contributions_are_committed_before_the_journey_is_reported() {
    let workflow = orders()
        .policy(WorkflowPolicyDescriptor::new("settle", || Settle).on_workflow_success())
        .step(charge_succeeding());

    let (status, events) = travel_workflow(workflow);

    assert_eq!(data_of(&status), [("balance".to_string(), 42)]);
    assert_eq!(
        kinds(&events)[events.len() - 3..],
        ["hook_called", "contribution_committed", "journey_succeeded"]
    );
}

#[test]
fn no_workflow_hook_runs_when_the_journey_is_aborted() {
    let saw = Seen::default();
    let workflow = closing(&saw).step(charge_succeeding().policy(broken()));

    let (status, _) = travel_workflow(workflow);

    assert!(matches!(status, JourneyStatus::Aborted(_)));
    assert!(seen(&saw).is_empty());
}

#[cfg(feature = "async")]
mod asynchronous {
    use super::*;
    use crate::engine::fixtures::{async_orders, travel};
    use crate::mode::Asynchronous;
    use crate::policy::AsyncOnWorkflowFailure;

    /// `on workflow failure` of an asynchronous workflow, which awaits its own event.
    struct Regret;

    impl AsyncOnWorkflowFailure<Orders> for Regret {
        fn needs() -> HookNeeds<WorkflowFailure> {
            HookNeeds::new().reporter()
        }

        async fn on_workflow_failure(
            &self,
            mut got: Requested<'_, Orders, WorkflowFailure, Asynchronous>,
        ) -> Result<(), Error> {
            got.reporter()?.warning("declined").await?;
            Ok(())
        }
    }

    #[test]
    fn an_asynchronous_workflow_hook_is_awaited_before_the_journey_is_reported() {
        let workflow = async_orders()
            .policy(WorkflowPolicyDescriptor::new_async("regret", || Regret).on_workflow_failure())
            .step(StepDescriptor::new_async(CHARGE, async || declined()))
            .build()
            .unwrap();

        let (status, events) = travel(workflow.instance(Orders).create().unwrap());

        assert!(matches!(status, JourneyStatus::Failed { .. }));
        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                "step_failed",
                "step_given_up",
                "journey_warning",
                "hook_called",
                "journey_failed"
            ]
        );
    }
}
