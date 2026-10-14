use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rstest::rstest;

use crate::engine::fixtures::{timed_out, travel};
use crate::error::Error;
use crate::event::Event;
use crate::journey::{Abort, JourneyStatus, MissingData, Requester};
use crate::policy::HookNeeds;
use crate::service::fixtures::{
    Given, Labeller, Made, PAYMENTS, given, implementations, made, per_attempt, per_journey,
    requesting,
};
use crate::service::{
    Implementation, PerJourney, Provided, ProviderName, ServiceProviderDescriptor, provide,
};
use crate::step::Input;
use crate::workflow::WorkflowBuilder;
use crate::workflow::fixtures::{Orders, orders};

const GATEWAY: Input<String> = Input::new("gateway");

struct Sandbox;

impl Labeller for Sandbox {
    fn label(&self) -> String {
        "sandbox".to_owned()
    }
}

fn sandbox() -> Result<Provided<dyn Labeller>, Error> {
    Ok(provide!(Sandbox))
}

fn counted(calls: &AtomicUsize) -> Result<Provided<dyn Labeller>, Error> {
    calls.fetch_add(1, Ordering::SeqCst);
    sandbox()
}

fn down() -> Result<Provided<dyn Labeller>, Error> {
    Err(Error::msg("the sandbox is down"))
}

/// Runs a journey of `workflow` with `fake` in place of the provider named payments.
fn faking(
    workflow: WorkflowBuilder<Orders>,
    fake: impl Fn() -> Result<Provided<dyn Labeller>, Error> + Send + Sync + 'static,
) -> (JourneyStatus, Vec<Event>) {
    let instance = workflow
        .build()
        .unwrap()
        .instance(Orders)
        .fake_provider::<dyn Labeller>("payments", fake)
        .create()
        .unwrap();
    travel(instance)
}

/// The orders workflow with one step, charge, which requests the payments service from a
/// provider of attempt scope and records the label of each instance in `charged`.
fn charging(charged: &Given, made_payments: &Made) -> WorkflowBuilder<Orders> {
    orders()
        .step(requesting("charge", &[&PAYMENTS], &[], charged))
        .provider(per_attempt("payments", &PAYMENTS, made_payments))
}

#[test]
fn a_fake_supplies_the_steps_its_provider_serves_and_service_provided_names_it() {
    let (made_payments, charged) = (Made::default(), Given::default());
    let (status, events) = faking(charging(&charged, &made_payments), sandbox);

    assert!(matches!(status, JourneyStatus::Succeeded { .. }));
    assert_eq!(made(&made_payments), 0);
    assert_eq!(given(&charged), ["sandbox"]);
    assert_eq!(
        implementations(&events),
        [(Implementation::of_type::<Sandbox>(), None)]
    );
}

type Charging = fn() -> WorkflowBuilder<Orders>;

/// A charge step that times out once and has a retry, served by a provider of journey scope.
fn charging_per_journey() -> WorkflowBuilder<Orders> {
    orders()
        .step(requesting("charge", &[&PAYMENTS], &[timed_out], &Given::default()).retry_budget(1))
        .provider(per_journey("payments", &PAYMENTS, &Made::default()))
}

/// A charge step that times out once and has a retry, served by a provider of attempt scope.
fn charging_per_attempt() -> WorkflowBuilder<Orders> {
    orders()
        .step(requesting("charge", &[&PAYMENTS], &[timed_out], &Given::default()).retry_budget(1))
        .provider(per_attempt("payments", &PAYMENTS, &Made::default()))
}

#[rstest]
#[case::of_journey_scope(charging_per_journey, 1)]
#[case::of_attempt_scope(charging_per_attempt, 2)]
fn a_fake_is_called_as_often_as_its_provider_would_be(
    #[case] charging: Charging,
    #[case] calls: usize,
) {
    let fake_calls = Made::default();
    let counting = Arc::clone(&fake_calls);

    faking(charging(), move || counted(&counting));

    assert_eq!(made(&fake_calls), calls);
}

/// A provider that requests the gateway from the workflow's data.
fn gateways() -> ServiceProviderDescriptor<Orders, PerJourney> {
    ServiceProviderDescriptor::new("payments", &PAYMENTS, |_: &Orders, _| sandbox())
        .needing(HookNeeds::new().from_workflow(&GATEWAY))
}

#[test]
fn the_requests_of_a_faked_provider_are_still_resolved_before_its_fake_is_called() {
    let fake_calls = Made::default();
    let counting = Arc::clone(&fake_calls);
    let workflow = orders()
        .step(requesting("charge", &[&PAYMENTS], &[], &Given::default()))
        .provider(gateways());

    let (status, _) = faking(workflow, move || counted(&counting));

    assert!(
        matches!(
            &status,
            JourneyStatus::Aborted(Abort::RequiredDataMissing(MissingData::Key {
                key,
                requester: Requester::Provider { provider }
            })) if key == "gateway" && *provider == ProviderName::from("payments")
        ),
        "{status:?}"
    );
    assert_eq!(made(&fake_calls), 0);
}

#[test]
fn a_fake_that_fails_aborts_the_journey_as_its_step_could_not_be_built() {
    let charged = Given::default();
    let (status, _) = faking(charging(&charged, &Made::default()), down);

    let JourneyStatus::Aborted(Abort::StepCouldNotBeBuilt(error)) = status else {
        panic!("{status:?}");
    };
    assert_eq!(error.to_string(), "the sandbox is down");
    assert!(given(&charged).is_empty());
}

#[test]
#[should_panic(
    expected = "the fake of the service provider payments supplies i64, but the provider supplies"
)]
fn a_fake_of_another_type_of_service_than_its_providers_panics() {
    let workflow = charging(&Given::default(), &Made::default())
        .build()
        .unwrap();

    workflow
        .instance(Orders)
        .fake_provider::<i64>("payments", || Ok(provide!(7_i64)));
}
