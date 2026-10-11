use std::sync::{Arc, Mutex};

use rstest::rstest;

use super::*;
use crate::engine::fixtures::{
    CHARGE, SHIP, Seen, broken, charge_succeeding, data_of, declined, kinds, seen, succeed,
    travel_workflow,
};
use crate::error::Error;
use crate::event::Event;
use crate::executor::LocalExecutor;
use crate::executor::fixtures::{Recorder, Shop, shop_builder};
use crate::policy::{
    HookNeeds, OnWorkflowFailure, OnWorkflowSuccess, Provides, Requested, WorkflowFailure,
    WorkflowPolicyDescriptor, WorkflowSuccess,
};
use crate::report::fixtures::entries;
use crate::step::Outcome;
use crate::workflow::WorkflowBuilder;
use crate::workflow::fixtures::{Orders, orders};

fn kind_and_step(event: &Event) -> (&'static str, Option<StepName>) {
    let step = match &event.body {
        EventBody::AttemptStarted { step } | EventBody::StepSucceeded { step } => Some(step.step),
        _ => None,
    };
    (event.kind(), step)
}

#[test]
fn steps_run_in_the_order_they_were_added() {
    let workflow = orders()
        .step(StepDescriptor::new(CHARGE, succeed))
        .step(StepDescriptor::new(SHIP, succeed));

    let (_, events) = travel_workflow(workflow);

    let stream: Vec<_> = events.iter().map(kind_and_step).collect();
    assert_eq!(
        stream,
        [
            ("journey_started", None),
            ("attempt_started", Some(CHARGE)),
            ("step_succeeded", Some(CHARGE)),
            ("attempt_started", Some(SHIP)),
            ("step_succeeded", Some(SHIP)),
            ("journey_succeeded", None),
        ]
    );
}

fn count(runs: &Mutex<u32>) -> Result<Outcome, Error> {
    *runs.lock().unwrap() += 1;
    Ok(Outcome::success())
}

#[test]
fn the_step_runs_once() {
    let runs = Arc::new(Mutex::new(0));
    let counted = Arc::clone(&runs);
    let workflow = shop_builder()
        .step(StepDescriptor::new(StepName::new("charge"), move || {
            count(&counted)
        }))
        .build()
        .unwrap();

    let result = LocalExecutor::new()
        .run(workflow.instance(Shop::new()).create().unwrap())
        .unwrap();

    assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
    assert_eq!(*runs.lock().unwrap(), 1);
}

#[test]
fn a_journey_without_a_step_succeeds() {
    let shop = Shop::new();
    let log = Arc::clone(&shop.log);
    let workflow = shop_builder().reporter::<Recorder<0>>().build().unwrap();

    let result = LocalExecutor::new()
        .run(workflow.instance(shop).create().unwrap())
        .unwrap();

    assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
    assert_eq!(
        entries(&log),
        ["audit journey_started", "audit journey_succeeded"]
    );
}

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
