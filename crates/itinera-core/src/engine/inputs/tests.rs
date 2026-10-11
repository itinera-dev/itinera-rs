use std::sync::{Arc, Mutex};

use rstest::rstest;

use super::*;
use crate::engine::fixtures::{
    CHARGE, Charge, Data, Read, SHIP, amount_as_i32, charged, instance, kinds, reads, travel,
    travel_with_amount, travel_workflow,
};
use crate::error::Error;
use crate::event::{self, Event, JourneyAbort, RequestSource};
use crate::instance::InstanceBuilder;
use crate::journey::{Abort, JourneyStatus, MissingData, Read as Found, Requester};
use crate::policy::{HookNeeds, InputAdapter};
use crate::step::{Input, OptionalInput, StepDescriptor, StepName};
use crate::workflow::fixtures::Orders;
use crate::workflow::{AdapterName, InputAdapterDescriptor, WorkflowBuilder};

/// Supplies an amount of 7, and nothing else.
fn pricing(
    _: &Orders,
    got: Requested<'_, Orders, InputAdapter>,
) -> Result<Option<AnyValue>, Error> {
    Ok((got.key() == "amount").then(|| AnyValue::new(7_i64)))
}

#[test]
fn a_step_is_built_with_the_inputs_it_declares_read_from_the_data_bag() {
    let read = Read::default();
    let journey = instance(charged(&read))
        .data("amount", 42_i64)
        .data("discount", 5_i64);

    let (status, events) = travel(journey.create().unwrap());

    assert!(matches!(status, JourneyStatus::Succeeded { .. }));
    assert_eq!(reads(&read), [(42, Some(5))]);
    assert_eq!(
        kinds(&events),
        [
            "journey_started",
            "attempt_started",
            "step_succeeded",
            "journey_succeeded"
        ]
    );
}

#[test]
fn an_input_adapter_supplies_an_input_before_the_data_bag_is_read() {
    let read = Read::default();
    let workflow =
        charged(&read).input_adapter(InputAdapterDescriptor::new("pricing", CHARGE, pricing));

    let (_, events) = travel_with_amount(workflow);

    assert_eq!(reads(&read), [(7, None)]);
    assert_eq!(
        kinds(&events),
        [
            "journey_started",
            "attempt_started",
            "input_adapter_supplied",
            "optional_input_absent",
            "step_succeeded",
            "journey_succeeded"
        ]
    );
}

#[test]
fn an_input_adapter_that_fails_aborts_the_journey_as_its_step_could_not_be_built() {
    let read = Read::default();
    let failing = InputAdapterDescriptor::new("pricing", CHARGE, |_: &Orders, _| {
        Err(Error::msg("no price list"))
    });

    let (status, events) = travel_workflow(charged(&read).input_adapter(failing));

    let JourneyStatus::Aborted(Abort::StepCouldNotBeBuilt(error)) = status else {
        panic!("the journey was not aborted as its step could not be built: {status:?}");
    };
    assert_eq!(error.to_string(), "no price list");
    assert_eq!(
        kinds(&events),
        [
            "journey_started",
            "attempt_started",
            "input_adapter_failed",
            "journey_aborted"
        ]
    );
}

#[test]
fn a_value_of_another_type_from_an_input_adapter_aborts_the_journey_with_wrong_type() {
    let read = Read::default();
    let wrong = InputAdapterDescriptor::new("pricing", CHARGE, |_: &Orders, _| {
        Ok(Some(AnyValue::new("seven".to_string())))
    });

    let (status, events) = travel_workflow(charged(&read).input_adapter(wrong));

    let JourneyStatus::Aborted(Abort::WrongType { key, requester }) = status else {
        panic!("the journey was not aborted for a wrong type: {status:?}");
    };
    assert_eq!(key, "amount");
    assert_eq!(
        requester,
        Requester::Adapter {
            adapter: AdapterName::from("pricing")
        }
    );
    assert_eq!(
        kinds(&events),
        [
            "journey_started",
            "attempt_started",
            "input_adapter_supplied",
            "journey_aborted"
        ]
    );
}

fn wrong_type_naming_the_step(status: &JourneyStatus) -> bool {
    matches!(
        status,
        JourneyStatus::Aborted(Abort::WrongType { key, requester: Requester::Step })
            if key == "amount"
    )
}

fn missing_data_naming_the_step(status: &JourneyStatus) -> bool {
    matches!(
        status,
        JourneyStatus::Aborted(Abort::RequiredDataMissing(MissingData::Key {
            key,
            requester: Requester::Step
        })) if key == "amount"
    )
}

#[rstest]
#[case::a_value_of_another_type_in_the_data_bag(amount_as_i32, wrong_type_naming_the_step)]
#[case::no_value_in_the_data_bag(std::convert::identity, missing_data_naming_the_step)]
fn an_input_adapter_that_returned_nothing_is_not_named_in_the_abort(
    #[case] data: Data,
    #[case] aborted: fn(&JourneyStatus) -> bool,
) {
    let read = Read::default();
    let silent = InputAdapterDescriptor::new("pricing", CHARGE, |_: &Orders, _| Ok(None));
    let workflow = charged(&read).input_adapter(silent);

    let (status, events) = travel(data(instance(workflow)).create().unwrap());

    assert!(aborted(&status), "{status:?}");
    assert!(reads(&read).is_empty());
    assert_eq!(
        kinds(&events),
        ["journey_started", "attempt_started", "journey_aborted"]
    );
}

const PRICE: Input<i64> = Input::new("price");
const QUANTITY: OptionalInput<i64> = OptionalInput::new("quantity");

/// Supplies the amount: the price from the workflow, times the quantity when there is one.
fn priced(
    _: &Orders,
    mut got: Requested<'_, Orders, InputAdapter>,
) -> Result<Option<AnyValue>, Error> {
    let price = got.from_workflow(&PRICE)?;
    let amount = price * got.optional_from_workflow(&QUANTITY)?.unwrap_or(1);
    Ok((got.key() == "amount").then(|| AnyValue::new(amount)))
}

/// The charge step, adapted by `priced`.
fn charged_at_the_price(read: &Read) -> WorkflowBuilder<Orders> {
    let pricing = InputAdapterDescriptor::new("pricing", CHARGE, priced).needing(
        HookNeeds::new()
            .from_workflow(&PRICE)
            .optional_from_workflow(&QUANTITY),
    );
    charged(read).input_adapter(pricing)
}

#[test]
fn an_input_adapter_receives_the_data_from_the_workflow_it_requests() {
    let read = Read::default();

    let journey = instance(charged_at_the_price(&read))
        .data("price", 6_i64)
        .data("quantity", 2_i64);
    let (_, events) = travel(journey.create().unwrap());

    assert_eq!(reads(&read), [(12, None)]);
    assert_eq!(
        kinds(&events),
        [
            "journey_started",
            "attempt_started",
            "input_adapter_supplied",
            "optional_input_absent",
            "step_succeeded",
            "journey_succeeded"
        ]
    );
}

#[test]
fn an_input_adapters_absent_optional_data_is_reported_before_what_it_supplied() {
    let read = Read::default();

    let journey = instance(charged_at_the_price(&read)).data("price", 7_i64);
    let (_, events) = travel(journey.create().unwrap());

    assert_eq!(reads(&read), [(7, None)]);
    let Some(Event {
        body: EventBody::OptionalInputAbsent { key, requester },
        ..
    }) = events.get(2)
    else {
        panic!("no optional_input_absent after attempt_started: {events:?}");
    };
    assert_eq!(key, "quantity");
    assert_eq!(
        *requester,
        RequestSource::Adapter {
            adapter: AdapterName::from("pricing"),
            step: StepAttempt::first(CHARGE),
        }
    );
    assert_eq!(
        kinds(&events),
        [
            "journey_started",
            "attempt_started",
            "optional_input_absent",
            "input_adapter_supplied",
            "optional_input_absent",
            "optional_input_absent",
            "step_succeeded",
            "journey_succeeded"
        ]
    );
}

fn price_as_text(journey: InstanceBuilder<Orders>) -> InstanceBuilder<Orders> {
    journey.data("price", "six".to_string())
}

fn wrong_type_naming_the_adapter(status: &JourneyStatus) -> bool {
    matches!(
        status,
        JourneyStatus::Aborted(Abort::WrongType { key, requester: Requester::Adapter { adapter } })
            if key == "price" && *adapter == AdapterName::from("pricing")
    )
}

fn missing_data_naming_the_adapter(status: &JourneyStatus) -> bool {
    matches!(
        status,
        JourneyStatus::Aborted(Abort::RequiredDataMissing(MissingData::Key {
            key,
            requester: Requester::Adapter { adapter }
        })) if key == "price" && *adapter == AdapterName::from("pricing")
    )
}

#[rstest]
#[case::a_value_of_another_type_in_the_data_bag(price_as_text, wrong_type_naming_the_adapter)]
#[case::no_value_in_the_data_bag(std::convert::identity, missing_data_naming_the_adapter)]
fn an_input_adapters_required_data_from_the_workflow_aborts_the_journey_before_it_runs(
    #[case] data: Data,
    #[case] aborted: fn(&JourneyStatus) -> bool,
) {
    let read = Read::default();

    let (status, events) = travel(
        data(instance(charged_at_the_price(&read)))
            .create()
            .unwrap(),
    );

    assert!(aborted(&status), "{status:?}");
    assert!(reads(&read).is_empty());
    assert_eq!(
        kinds(&events),
        ["journey_started", "attempt_started", "journey_aborted"]
    );
    assert_eq!(
        events.last().and_then(aborting_requester),
        Some(&event::Requester::Adapter {
            adapter: AdapterName::from("pricing"),
            step: CHARGE,
        })
    );
}

/// Who made the request that aborted the journey, as `journey_aborted` names it.
fn aborting_requester(event: &Event) -> Option<&event::Requester> {
    match &event.body {
        EventBody::JourneyAborted {
            abort:
                JourneyAbort::RequiredDataMissing {
                    missing: event::MissingData::Key { requester, .. },
                }
                | JourneyAbort::WrongType { requester, .. },
        } => Some(requester),
        _ => None,
    }
}

/// The names of the steps an input adapter was told it supplies, in order.
type Names = Arc<Mutex<Vec<StepName>>>;

/// Records the name of the step it supplies an input to, and supplies nothing.
fn naming(
    names: &Names,
) -> impl for<'a> Fn(&'a Orders, Requested<'a, Orders, InputAdapter>) -> Result<Option<AnyValue>, Error>
+ Send
+ Sync
+ 'static {
    let names = Arc::clone(names);
    move |_, got| {
        names.lock().unwrap().push(got.step_name());
        Ok(None)
    }
}

#[test]
fn an_input_adapter_is_told_the_step_it_supplies_for_each_of_its_inputs() {
    let names = Names::default();
    let read = Read::default();
    let ship = Charge {
        read: Arc::clone(&read),
    };
    let workflow = charged(&read)
        .step(StepDescriptor::new(SHIP, ship))
        .input_adapter(InputAdapterDescriptor::new("naming", CHARGE, naming(&names)).step(SHIP));

    travel(instance(workflow).data("amount", 42_i64).create().unwrap());

    assert_eq!(*names.lock().unwrap(), [CHARGE, CHARGE, SHIP, SHIP]);
}

/// Supplies the amount as twice the base read through its access to the data bag.
fn doubled(
    _: &Orders,
    mut got: Requested<'_, Orders, InputAdapter>,
) -> Result<Option<AnyValue>, Error> {
    let base = got.data_bag()?.read::<i64>("base");
    match (got.key(), base) {
        ("amount", Found::Present(base)) => Ok(Some(AnyValue::new(base * 2))),
        _ => Ok(None),
    }
}

#[test]
fn an_input_adapter_reads_the_data_bag_without_events() {
    let read = Read::default();
    let workflow = charged(&read).input_adapter(
        InputAdapterDescriptor::new("pricing", CHARGE, doubled)
            .needing(HookNeeds::new().data_bag()),
    );

    let (_, events) = travel(instance(workflow).data("base", 5_i64).create().unwrap());

    assert_eq!(reads(&read), [(10, None)]);
    assert_eq!(
        kinds(&events),
        [
            "journey_started",
            "attempt_started",
            "input_adapter_supplied",
            "optional_input_absent",
            "step_succeeded",
            "journey_succeeded"
        ]
    );
}

#[test]
fn an_input_adapter_that_did_not_declare_the_data_bag_cannot_read_it() {
    let read = Read::default();
    let workflow =
        charged(&read).input_adapter(InputAdapterDescriptor::new("pricing", CHARGE, doubled));

    let (status, _) = travel(instance(workflow).data("base", 5_i64).create().unwrap());

    assert!(matches!(
        status,
        JourneyStatus::Aborted(Abort::StepCouldNotBeBuilt(_))
    ));
}
