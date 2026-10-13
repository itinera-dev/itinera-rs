use std::sync::{Arc, Mutex};

use super::*;
use crate::engine::fixtures::{
    CHARGE, failing_on, instance, kinds, timed_out, travel, travel_before, travel_workflow,
};
use crate::event::Event;
use crate::journey::{DataBag, JourneyStatus, MissingData, Requester};
use crate::policy::{HookNeeds, Provider, Requested};
use crate::service::fixtures::{
    Given, Label, Labeller, Made, PAYMENTS, STOCK, given, labelled, made, per_attempt, per_journey,
    requesting,
};
use crate::service::{
    Implementation, PerAttempt, PerJourney, Provided, ServiceProviderDescriptor, provide,
};
use crate::step::{Input, OptionalInput};
use crate::value::AnyValue;
use crate::workflow::WorkflowBuilder;
use crate::workflow::fixtures::{Orders, orders};

const GATEWAY: Input<String> = Input::new("gateway");
const REGION: OptionalInput<String> = OptionalInput::new("region");

/// The step, attempt and provider of every `service_provided`, in order.
fn provided(events: &[Event]) -> Vec<(StepName, u32, ProviderName)> {
    events.iter().filter_map(provision).collect()
}

fn provision(event: &Event) -> Option<(StepName, u32, ProviderName)> {
    match &event.body {
        EventBody::ServiceProvided { step, provider, .. } => {
            Some((step.step, step.attempt.get(), *provider))
        }
        _ => None,
    }
}

fn the_orders_journey_with(builder: WorkflowBuilder<Orders>) -> (JourneyStatus, Vec<Event>) {
    travel(
        instance(builder)
            .data("gateway", "main".to_string())
            .create()
            .unwrap(),
    )
}

/// The orders workflow with one step, charge, which requests the payments service and records
/// the label of each instance in `given`.
fn charging(given: &Given) -> WorkflowBuilder<Orders> {
    orders().step(requesting("charge", &[&PAYMENTS], &[], given))
}

#[test]
fn a_journey_scoped_service_is_made_once_and_lent_to_every_step_that_requests_it() {
    let (made_stock, reserved, shipped) = (Made::default(), Given::default(), Given::default());
    let workflow = orders()
        .step(requesting("reserve", &[&STOCK], &[], &reserved))
        .step(requesting("ship", &[&STOCK], &[], &shipped))
        .provider(per_journey("stock", &STOCK, &made_stock));

    let (status, events) = travel_workflow(workflow);

    assert!(matches!(status, JourneyStatus::Succeeded { .. }));
    assert_eq!(made(&made_stock), 1);
    assert_eq!(given(&reserved), ["stock 1"]);
    assert_eq!(given(&shipped), ["stock 1"]);
    assert_eq!(
        provided(&events),
        [(StepName::new("reserve"), 1, ProviderName::from("stock"))]
    );
}

#[test]
fn an_attempt_scoped_service_is_made_for_every_attempt_retries_included() {
    let (made_payments, charged) = (Made::default(), Given::default());
    let workflow = orders()
        .step(requesting("charge", &[&PAYMENTS], &[timed_out], &charged).retry_budget(1))
        .provider(per_attempt("payments", &PAYMENTS, &made_payments));

    let (_, events) = travel_workflow(workflow);

    assert_eq!(made(&made_payments), 2);
    assert_eq!(given(&charged), ["payments 1", "payments 2"]);
    assert_eq!(
        provided(&events),
        [
            (CHARGE, 1, ProviderName::from("payments")),
            (CHARGE, 2, ProviderName::from("payments")),
        ]
    );
}

#[test]
fn a_journey_scoped_service_serves_every_attempt_of_a_retried_step() {
    let (made_stock, charged) = (Made::default(), Given::default());
    let workflow = orders()
        .step(requesting("charge", &[&STOCK], &[timed_out], &charged).retry_budget(1))
        .provider(per_journey("stock", &STOCK, &made_stock));

    travel_workflow(workflow);

    assert_eq!(made(&made_stock), 1);
    assert_eq!(given(&charged), ["stock 1", "stock 1"]);
}

#[test]
fn a_journey_scoped_provider_attached_to_steps_shares_its_instance_among_those_steps_only() {
    let (made_shared, made_payments) = (Made::default(), Made::default());
    let (reserved, charged, shipped) = (Given::default(), Given::default(), Given::default());
    let workflow = orders()
        .step(requesting("reserve", &[&PAYMENTS], &[], &reserved))
        .step(requesting("charge", &[&PAYMENTS], &[], &charged))
        .step(requesting("ship", &[&PAYMENTS], &[], &shipped))
        .provider(per_journey("payments", &PAYMENTS, &made_payments))
        .provider(
            per_journey("shared", &PAYMENTS, &made_shared)
                .step(StepName::new("reserve"))
                .step(StepName::new("ship")),
        );

    travel_workflow(workflow);

    assert_eq!(made(&made_shared), 1);
    assert_eq!(given(&reserved), ["shared 1"]);
    assert_eq!(given(&charged), ["payments 1"]);
    assert_eq!(given(&shipped), ["shared 1"]);
}

/// The key and provider of every `service_provided`, in order.
fn provided_keys(events: &[Event]) -> Vec<(&str, ProviderName)> {
    events.iter().filter_map(provided_key).collect()
}

fn provided_key(event: &Event) -> Option<(&str, ProviderName)> {
    match &event.body {
        EventBody::ServiceProvided { key, provider, .. } => Some((key, *provider)),
        _ => None,
    }
}

#[test]
fn a_steps_services_are_supplied_in_the_order_it_declares_them() {
    let (made_services, charged) = (Made::default(), Given::default());
    let workflow = orders()
        .step(requesting("charge", &[&STOCK, &PAYMENTS], &[], &charged))
        .provider(per_attempt("payments", &PAYMENTS, &made_services))
        .provider(per_attempt("stock", &STOCK, &made_services));

    let (_, events) = travel_workflow(workflow);

    assert_eq!(given(&charged), ["stock 1", "payments 2"]);
    assert_eq!(
        provided_keys(&events),
        [
            ("stock", ProviderName::from("stock")),
            ("payments", ProviderName::from("payments")),
        ]
    );
}

struct Sandbox;

impl Labeller for Sandbox {
    fn label(&self) -> String {
        "sandbox".to_owned()
    }
}

fn sandbox(
    _: &Orders,
    _: Requested<'_, Orders, Provider<PerJourney>>,
) -> Result<Provided<dyn Labeller>, Error> {
    Ok(provide!(Sandbox, "north".to_owned()))
}

/// The implementation of every `service_provided`, in order, with the data it carries when that
/// is a string.
fn implementations(events: &[Event]) -> Vec<(Implementation, Option<String>)> {
    events.iter().filter_map(implementation).collect()
}

fn implementation(event: &Event) -> Option<(Implementation, Option<String>)> {
    match &event.body {
        EventBody::ServiceProvided {
            implementation,
            data,
            ..
        } => Some((*implementation, data.as_ref().and_then(as_string).cloned())),
        _ => None,
    }
}

fn as_string(data: &AnyValue) -> Option<&String> {
    data.downcast_ref()
}

#[test]
fn service_provided_names_the_type_each_provider_made_and_carries_the_data_it_supplied() {
    let workflow = orders()
        .step(requesting("reserve", &[&STOCK], &[], &Given::default()))
        .step(requesting("charge", &[&PAYMENTS], &[], &Given::default()))
        .provider(per_journey("stock", &STOCK, &Made::default()))
        .provider(ServiceProviderDescriptor::new(
            "sandbox", &PAYMENTS, sandbox,
        ));

    let (_, events) = travel_workflow(workflow);

    assert_eq!(
        implementations(&events),
        [
            (Implementation::of_type::<Label>(), None),
            (
                Implementation::of_type::<Sandbox>(),
                Some("north".to_owned())
            ),
        ]
    );
}

#[test]
fn a_provider_attached_to_a_step_that_does_not_request_its_service_is_never_called() {
    let made_payments = Made::default();
    let workflow = orders()
        .step(requesting("charge", &[], &[], &Given::default()))
        .provider(per_attempt("payments", &PAYMENTS, &made_payments).step(CHARGE));

    let (status, _) = travel_workflow(workflow);

    assert!(matches!(status, JourneyStatus::Succeeded { .. }));
    assert_eq!(made(&made_payments), 0);
}

/// What an attempt-scoped provider was told about each call: the step, the attempt and the
/// journey ID.
type Told = Arc<Mutex<Vec<(StepName, u32, String)>>>;

fn telling(told: &Told) -> ServiceProviderDescriptor<Orders, PerAttempt> {
    let told = Arc::clone(told);
    ServiceProviderDescriptor::per_attempt("payments", &PAYMENTS, move |_: &Orders, got| {
        tell(&told, &got)
    })
}

fn tell(
    told: &Told,
    got: &Requested<'_, Orders, Provider<PerAttempt>>,
) -> Result<Provided<dyn Labeller>, Error> {
    told.lock().unwrap().push((
        got.step_name(),
        got.attempt().get(),
        got.journey_id().to_string(),
    ));
    Ok(labelled("payments", &Made::default()))
}

fn order_7(_: &Orders, _: &DataBag) -> Result<String, Error> {
    Ok("order-7".to_string())
}

#[test]
fn an_attempt_scoped_provider_is_told_the_step_the_attempt_and_the_journey_id() {
    let told = Told::default();
    let workflow = orders()
        .step(requesting("charge", &[&PAYMENTS], &[timed_out], &Given::default()).retry_budget(1))
        .provider(telling(&told))
        .id_generator(order_7);

    travel_workflow(workflow);

    assert_eq!(
        *told.lock().unwrap(),
        [
            (CHARGE, 1, "order-7".to_string()),
            (CHARGE, 2, "order-7".to_string()),
        ]
    );
}

/// A provider of journey scope that labels its instance with the gateway it reads from the
/// workflow's data, and the region if there is one.
fn gateways() -> ServiceProviderDescriptor<Orders, PerJourney> {
    ServiceProviderDescriptor::new("gateways", &PAYMENTS, gateway).needing(
        HookNeeds::new()
            .from_workflow(&GATEWAY)
            .optional_from_workflow(&REGION),
    )
}

fn gateway(
    _: &Orders,
    mut got: Requested<'_, Orders, Provider<PerJourney>>,
) -> Result<Provided<dyn Labeller>, Error> {
    let gateway = got.from_workflow(&GATEWAY)?;
    let region = got.optional_from_workflow(&REGION)?.unwrap_or_default();
    Ok(labelled(&format!("{gateway}{region}"), &Made::default()))
}

#[test]
fn a_provider_is_given_the_data_it_requests_and_optional_data_absent_is_reported() {
    let charged = Given::default();
    let workflow = charging(&charged).provider(gateways());

    let (_, events) = the_orders_journey_with(workflow);

    assert_eq!(given(&charged), ["main 1"]);
    assert_eq!(
        kinds(&events),
        [
            "journey_started",
            "attempt_started",
            "optional_input_absent",
            "service_provided",
            "step_succeeded",
            "journey_succeeded"
        ]
    );
}

#[test]
fn a_providers_required_data_missing_aborts_the_journey_naming_the_provider() {
    let charged = Given::default();
    let workflow = charging(&charged).provider(gateways());

    let (status, events) = travel_workflow(workflow);

    assert!(
        matches!(
            &status,
            JourneyStatus::Aborted(Abort::RequiredDataMissing(MissingData::Key {
                key,
                requester: Requester::Provider { provider }
            })) if key == "gateway" && *provider == ProviderName::from("gateways")
        ),
        "{status:?}"
    );
    assert!(given(&charged).is_empty());
    assert_eq!(
        kinds(&events),
        ["journey_started", "attempt_started", "journey_aborted"]
    );
}

fn failing(
    _: &Orders,
    _: Requested<'_, Orders, Provider<PerAttempt>>,
) -> Result<Provided<dyn Labeller>, Error> {
    Err(Error::msg("the gateway is down"))
}

#[test]
fn a_provider_that_fails_aborts_the_journey_as_its_step_could_not_be_built() {
    let charged = Given::default();
    let workflow = charging(&charged).provider(ServiceProviderDescriptor::per_attempt(
        "payments", &PAYMENTS, failing,
    ));

    let (status, events) = travel_workflow(workflow);

    let JourneyStatus::Aborted(Abort::StepCouldNotBeBuilt(error)) = status else {
        panic!("{status:?}");
    };
    assert_eq!(error.to_string(), "the gateway is down");
    assert!(given(&charged).is_empty());
    assert_eq!(
        kinds(&events),
        [
            "journey_started",
            "attempt_started",
            "service_provider_failed",
            "journey_aborted"
        ]
    );
    assert!(events.iter().any(is_the_payments_failure));
    assert!(matches!(
        events.last().map(|event| &event.body),
        Some(EventBody::JourneyAborted {
            abort: JourneyAbort::StepCouldNotBeBuilt { provider: Some(provider), .. }
        }) if *provider == ProviderName::from("payments")
    ));
}

fn is_the_payments_failure(event: &Event) -> bool {
    matches!(
        &event.body,
        EventBody::ServiceProviderFailed { step, key, provider }
            if *step == StepAttempt::first(CHARGE)
                && key == "payments"
                && *provider == ProviderName::from("payments")
    )
}

fn announcing(
    _: &Orders,
    mut got: Requested<'_, Orders, Provider<PerAttempt>>,
) -> Result<Provided<dyn Labeller>, Error> {
    got.reporter()?.info("connecting")?;
    Ok(labelled("payments", &Made::default()))
}

fn announced() -> WorkflowBuilder<Orders> {
    charging(&Given::default()).provider(
        ServiceProviderDescriptor::per_attempt("payments", &PAYMENTS, announcing)
            .needing(HookNeeds::new().reporter()),
    )
}

#[test]
fn a_providers_events_name_it_and_come_before_its_service_is_provided() {
    let (_, events) = travel_workflow(announced());

    assert_eq!(
        kinds(&events),
        [
            "journey_started",
            "attempt_started",
            "journey_info",
            "service_provided",
            "step_succeeded",
            "journey_succeeded"
        ]
    );
    assert!(events.iter().any(is_the_providers_info));
}

fn is_the_providers_info(event: &Event) -> bool {
    matches!(
        &event.body,
        EventBody::JourneyInfo {
            hook: HookSource::Provider { provider, step },
            ..
        } if *provider == ProviderName::from("payments") && step.step == CHARGE
    )
}

#[test]
fn a_reporter_failing_on_a_providers_event_aborts_the_journey_before_its_service_is_provided() {
    let (status, events) = travel_before(
        instance(announced()).create().unwrap(),
        Some(failing_on("journey_info")),
    );

    assert!(
        matches!(status, JourneyStatus::Aborted(Abort::ReporterFailed(_))),
        "{status:?}"
    );
    assert_eq!(
        kinds(&events),
        [
            "journey_started",
            "attempt_started",
            "journey_info",
            "journey_aborted"
        ]
    );
}

#[cfg(feature = "async")]
mod asynchronous {
    use super::*;
    use crate::engine::fixtures::async_orders;
    use crate::mode::Asynchronous;
    use crate::step::{AsyncStep, AsyncStepFactory, Outcome, Resolved, StepNeeds};

    /// The factory of an asynchronous step that records the label of the stock it is given.
    struct Reserve {
        given: Given,
    }

    impl AsyncStepFactory for Reserve {
        type Step<'a> = Reserved;

        fn needs(&self) -> StepNeeds {
            StepNeeds::new().service(&STOCK)
        }

        fn build<'a>(&'a self, got: &mut Resolved<'a, Asynchronous>) -> Result<Reserved, Error> {
            self.given
                .lock()
                .unwrap()
                .push(got.service(&STOCK)?.label());
            Ok(Reserved)
        }
    }

    struct Reserved;

    impl AsyncStep for Reserved {
        async fn run(self) -> Result<Outcome, Error> {
            Ok(Outcome::success())
        }
    }

    async fn stock(
        _: &Orders,
        mut got: Requested<'_, Orders, Provider<PerJourney>, Asynchronous>,
    ) -> Result<Provided<dyn Labeller>, Error> {
        got.reporter()?.info("counting").await?;
        Ok(labelled("stock", &Made::default()))
    }

    #[test]
    fn an_asynchronous_provider_is_awaited_while_its_step_is_built() {
        let reserved = Given::default();
        let workflow = async_orders()
            .step(StepDescriptor::new_async(
                StepName::new("reserve"),
                Reserve {
                    given: Arc::clone(&reserved),
                },
            ))
            .provider(
                ServiceProviderDescriptor::new_async("stock", &STOCK, stock)
                    .needing(HookNeeds::new().reporter()),
            )
            .build()
            .unwrap();

        let (status, events) = travel(workflow.instance(Orders).create().unwrap());

        assert!(matches!(status, JourneyStatus::Succeeded { .. }));
        assert_eq!(given(&reserved), ["stock 1"]);
        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                "journey_info",
                "service_provided",
                "step_succeeded",
                "journey_succeeded"
            ]
        );
    }
}

mod rescue_steps {
    use super::*;
    use crate::service::Service;
    use crate::service::fixtures::rescued_requesting;

    const CHARGE_RESCUE: StepName = StepName::new("charge_rescue");

    /// The orders workflow with one step, charge, whose first attempt fails, and its rescue step;
    /// each requests the service under `service`'s key and records its labels.
    fn rescued_charge(
        service: &'static Service<dyn Labeller>,
        charged: &Given,
        rescued: &Given,
    ) -> WorkflowBuilder<Orders> {
        orders().step(rescued_requesting(
            requesting("charge", &[service], &[timed_out], charged),
            &[service],
            rescued,
        ))
    }

    #[test]
    fn a_rescue_step_is_served_by_the_provider_attached_to_no_step() {
        let (charged, rescued) = (Given::default(), Given::default());
        let workflow = rescued_charge(&PAYMENTS, &charged, &rescued)
            .provider(per_attempt("payments", &PAYMENTS, &Made::default()))
            .provider(per_attempt("charging", &PAYMENTS, &Made::default()).step(CHARGE));

        let (status, events) = travel_workflow(workflow);

        assert!(matches!(status, JourneyStatus::Succeeded { .. }));
        assert_eq!(given(&charged), ["charging 1", "charging 2"]);
        assert_eq!(given(&rescued), ["payments 1"]);
        assert_eq!(
            provided(&events),
            [
                (CHARGE, 1, ProviderName::from("charging")),
                (CHARGE_RESCUE, 1, ProviderName::from("payments")),
                (CHARGE, 2, ProviderName::from("charging")),
            ]
        );
    }

    #[test]
    fn an_attempt_scoped_provider_is_told_the_rescue_steps_name_and_its_attempt_1() {
        let told = Told::default();
        let workflow = orders()
            .step(rescued_requesting(
                requesting("charge", &[], &[timed_out], &Given::default()),
                &[&PAYMENTS],
                &Given::default(),
            ))
            .provider(telling(&told))
            .id_generator(order_7);

        travel_workflow(workflow);

        assert_eq!(
            *told.lock().unwrap(),
            [(CHARGE_RESCUE, 1, "order-7".to_string())]
        );
    }

    #[test]
    fn a_journey_scoped_provider_lends_a_rescue_step_the_journeys_instance() {
        let (made_stock, charged, rescued) = (Made::default(), Given::default(), Given::default());
        let workflow = rescued_charge(&STOCK, &charged, &rescued).provider(per_journey(
            "stock",
            &STOCK,
            &made_stock,
        ));

        travel_workflow(workflow);

        assert_eq!(made(&made_stock), 1);
        assert_eq!(given(&rescued), ["stock 1"]);
        assert_eq!(given(&charged), ["stock 1", "stock 1"]);
    }

    #[test]
    fn a_provider_attached_to_a_rescue_steps_name_serves_it_instead_of_the_one_attached_to_no_step()
    {
        let (charged, rescued) = (Given::default(), Given::default());
        let workflow = rescued_charge(&PAYMENTS, &charged, &rescued)
            .provider(per_attempt("payments", &PAYMENTS, &Made::default()))
            .provider(per_attempt("backup", &PAYMENTS, &Made::default()).step(CHARGE_RESCUE));

        let (status, events) = travel_workflow(workflow);

        assert!(matches!(status, JourneyStatus::Succeeded { .. }));
        assert_eq!(given(&charged), ["payments 1", "payments 2"]);
        assert_eq!(given(&rescued), ["backup 1"]);
        assert_eq!(
            provided(&events),
            [
                (CHARGE, 1, ProviderName::from("payments")),
                (CHARGE_RESCUE, 1, ProviderName::from("backup")),
                (CHARGE, 2, ProviderName::from("payments")),
            ]
        );
    }

    #[test]
    fn a_journey_scoped_provider_attached_to_a_step_and_its_rescue_step_lends_both_one_instance() {
        let (made_stock, made_warehouse) = (Made::default(), Made::default());
        let (charged, rescued) = (Given::default(), Given::default());
        let workflow = rescued_charge(&STOCK, &charged, &rescued)
            .provider(per_journey("stock", &STOCK, &made_stock))
            .provider(
                per_journey("warehouse", &STOCK, &made_warehouse)
                    .step(CHARGE)
                    .step(CHARGE_RESCUE),
            );

        travel_workflow(workflow);

        assert_eq!((made(&made_stock), made(&made_warehouse)), (0, 1));
        assert_eq!(given(&rescued), ["warehouse 1"]);
        assert_eq!(given(&charged), ["warehouse 1", "warehouse 1"]);
    }
}
