//! Workflows the macros declare, against the same workflows declared with the builder.

use std::sync::Mutex;

use itinera::error::Error;
use itinera::event::Event;
use itinera::executor::LocalExecutor;
use itinera::instance::WorkflowInstance;
use itinera::journey::{Contributor, DataBag, InitialData, JourneyId, Read};
use itinera::policy::{
    HookNeeds, InputAdapter, OnFailure, OnSuccess, Provides, Requested, StepPolicy, WorkflowPolicy,
    step_policy, workflow_policy,
};
use itinera::report::{GlobalMetadata, RecordingReporter, Reporter};
use itinera::step::{DescribedStep, OptionalInput, Outcome, Step, StepName, step, step_name};
use itinera::value::AnyValue;
use itinera::workflow::{
    InputAdapterDescriptor, Supplied, Workflow, WorkflowDescriptor, role, workflow,
};
use rstest::rstest;

use super::fixtures::{Journey, run_created};

/// A role of the shop: the till its receipts are printed at.
trait Till: Sync {
    fn till(&self) -> &'static str;
}

/// Charges the amount, which an input adapter may supply.
#[derive(Step)]
struct Charge {
    #[input]
    amount: i64,
}

#[step]
impl Charge {
    fn run(self, contributor: &mut Contributor) -> Result<Outcome, Error> {
        match self.amount {
            amount if amount < 0 => Err(Error::msg("the till cannot charge a negative amount")),
            0 => Self::fail("nothing-to-charge"),
            amount => {
                contributor.contribute("charged", amount);
                Self::succeed()
            }
        }
    }
}

#[derive(Step)]
struct Ship {}

#[step]
impl Ship {
    fn run(self) -> Result<Outcome, Error> {
        Self::succeed()
    }
}

#[derive(Default)]
struct Receipt;

#[step_policy(name = "receipt")]
impl Receipt {
    #[on_step_success]
    fn printed(
        &self,
        #[from_step] charged: i64,
        #[role] till: &dyn Till,
        contributor: &mut Contributor,
    ) -> Result<OnSuccess, Error> {
        contributor.contribute("receipt", format!("{charged} at the {}", till.till()));
        Self::keep_default()
    }
}

#[derive(Default)]
struct Watch;

#[step_policy(name = "watch")]
impl Watch {
    #[on_step_abnormal_termination]
    fn crashed(&self) -> Result<OnFailure, Error> {
        Self::fail_workflow("till-broken")
    }
}

#[derive(Default)]
struct Close;

#[workflow_policy(name = "close")]
impl Close {
    #[on_workflow_success]
    fn closed(&self, contributor: &mut Contributor) -> Result<(), Error> {
        contributor.contribute("closed", true);
        Ok(())
    }
}

#[derive(Default)]
struct Alarm;

#[workflow_policy(name = "alarm")]
impl Alarm {
    #[on_workflow_failure]
    fn raised(&self, contributor: &mut Contributor) -> Result<(), Error> {
        contributor.contribute("alarm", true);
        Ok(())
    }
}

/// The journeys each listed reporter saw, in the order it saw their events.
static TALLIED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// A listed reporter, made for every journey, which tallies the journeys it sees.
#[derive(Default)]
struct Tally {
    metadata: GlobalMetadata,
}

impl Reporter for Tally {
    fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<(), Error> {
        self.metadata.insert(key, value.clone());
        Ok(())
    }

    fn report(&mut self, event: &Event) -> Result<(), Error> {
        let journey_id = self
            .metadata
            .get(GlobalMetadata::JOURNEY_ID)
            .and_then(AnyValue::downcast_ref::<String>);
        let seen = format!("{journey_id:?} {}", event.sequence);
        TALLIED.lock().unwrap().push(seen);
        Ok(())
    }
}

#[workflow(name = "shop", policies = [Close, Alarm], reporters = [Tally])]
#[with_step(Charge, name = "charge", retry_budget = 1, no_retry_on_abnormal_termination, policies = [Receipt, Watch])]
#[with_step(Ship, name = "ship")]
#[with_step(Ship, name = "ship again")]
struct Shop {
    till: &'static str,
    #[default]
    fee: i64,
    region: Option<&'static str>,
}

#[role]
impl Till for Shop {
    fn till(&self) -> &'static str {
        self.till
    }
}

#[workflow]
impl Shop {
    #[input_adapter(steps = ["charge"], name = "pricing")]
    fn priced(
        &self,
        #[key] key: &str,
        #[from_workflow] price: Option<i64>,
        step: StepName,
        journey: &JourneyId,
        data: &DataBag,
    ) -> Result<Supplied, Error> {
        priced(self.fee, key, price, step, journey, data)
    }

    #[id_generator]
    fn basket_id(&self, data: &DataBag) -> Result<String, Error> {
        basket_id(self.region, data)
    }
}

/// The amount of a basket: its price with the shop's fee, or else the amount in the data bag.
fn priced(
    fee: i64,
    key: &str,
    price: Option<i64>,
    step: StepName,
    journey: &JourneyId,
    data: &DataBag,
) -> Result<Supplied, Error> {
    if step != step_name!("charge") || key != "amount" {
        return Err(Error::msg(format!("{journey} asked for {key} of {step}")));
    }
    match (price, data.read::<i64>("amount")) {
        (Some(price), _) => Ok(Supplied::value(price + fee)),
        (None, Read::Present(amount)) => Ok(Supplied::value(amount)),
        (None, _) => Ok(Supplied::FromDataBag),
    }
}

fn basket_id(region: Option<&str>, data: &DataBag) -> Result<String, Error> {
    let basket = data.optional::<String>("basket")?.unwrap_or_default();
    Ok(format!("{}-{basket}", region.unwrap_or("nowhere")))
}

/// The same shop, declared with the builder.
struct HandShop {
    till: &'static str,
    fee: i64,
    region: Option<&'static str>,
}

impl Till for HandShop {
    fn till(&self) -> &'static str {
        self.till
    }
}

impl Provides<dyn Till> for HandShop {
    fn role(&self) -> &(dyn Till + 'static) {
        self
    }
}

const PRICE: OptionalInput<i64> = OptionalInput::new("price");

fn hand_priced(
    shop: &HandShop,
    mut got: Requested<'_, HandShop, InputAdapter>,
) -> Result<Supplied, Error> {
    let price = got.optional_from_workflow(&PRICE)?;
    let data = got.data_bag()?;
    priced(
        shop.fee,
        got.key(),
        price,
        got.step_name(),
        got.journey_id(),
        data,
    )
}

fn hand_basket_id(shop: &HandShop, data: &DataBag) -> Result<String, Error> {
    basket_id(shop.region, data)
}

fn by_hand() -> WorkflowDescriptor<HandShop> {
    let charge = Charge::descriptor(step_name!("charge"))
        .retry_budget(1)
        .no_retry_on_abnormal_termination()
        .policy(Receipt::descriptor())
        .policy(Watch::descriptor());
    let pricing = InputAdapterDescriptor::new("pricing", step_name!("charge"), hand_priced)
        .needing(HookNeeds::new().optional_from_workflow(&PRICE).data_bag());
    WorkflowDescriptor::builder("shop")
        .step(charge)
        .step(Ship::descriptor(step_name!("ship")))
        .step(Ship::descriptor(step_name!("ship again")))
        .policy(Close::descriptor())
        .policy(Alarm::descriptor())
        .reporter::<Tally>()
        .input_adapter(pricing)
        .id_generator(hand_basket_id)
        .build()
        .unwrap()
}

/// The initial data of a journey of the shop.
#[derive(Clone, InitialData)]
struct Basket {
    basket: String,
    price: Option<i64>,
    amount: Option<i64>,
}

fn a_priced_basket() -> Basket {
    Basket {
        basket: "priced".to_owned(),
        price: Some(40),
        amount: None,
    }
}

fn with_an_amount() -> Basket {
    Basket {
        basket: "with an amount".to_owned(),
        price: None,
        amount: Some(12),
    }
}

fn empty() -> Basket {
    Basket {
        basket: "empty".to_owned(),
        price: Some(0),
        amount: None,
    }
}

fn negative() -> Basket {
    Basket {
        basket: "negative".to_owned(),
        price: Some(-3),
        amount: None,
    }
}

fn unpriced() -> Basket {
    Basket {
        basket: "unpriced".to_owned(),
        price: None,
        amount: None,
    }
}

fn by_macros(basket: Basket) -> Journey {
    run_created(Shop::descriptor(), |recorder| {
        Shop::builder()
            .till("front till")
            .region(Some("north"))
            .initial_data(basket)
            .reporter(recorder)
            .create()
            .unwrap()
    })
}

fn by_the_builder(basket: Basket) -> Journey {
    let shop = HandShop {
        till: "front till",
        fee: 0,
        region: Some("north"),
    };
    let workflow = by_hand();
    run_created(&workflow, |recorder| {
        workflow
            .instance(shop)
            .initial_data(basket)
            .reporter(recorder)
            .create()
            .unwrap()
    })
}

#[rstest]
#[case::priced(a_priced_basket())]
#[case::with_an_amount(with_an_amount())]
#[case::empty(empty())]
#[case::negative(negative())]
#[case::unpriced(unpriced())]
fn a_workflow_the_macros_declare_lists_and_runs_as_one_declared_with_the_builder(
    #[case] basket: Basket,
) {
    assert_eq!(by_macros(basket.clone()), by_the_builder(basket));
}

#[test]
fn a_workflow_is_declared_once_and_shared_by_every_instance() {
    assert!(std::ptr::eq(Shop::descriptor(), Shop::descriptor()));
}

#[test]
fn a_field_left_unset_starts_from_its_default() {
    let instance = Shop::builder().till("back till").create().unwrap();
    let shop = instance.workflow();
    assert_eq!((shop.till, shop.fee, shop.region), ("back till", 0, None));
}

fn tallied(basket: &str) {
    let basket = Basket {
        basket: basket.to_owned(),
        ..a_priced_basket()
    };
    let instance = Shop::builder()
        .till("front till")
        .region(Some("south"))
        .initial_data(basket)
        .create()
        .unwrap();
    let result = LocalExecutor::new().run(instance).unwrap();
    assert!(result.status.is_succeeded());
}

#[test]
fn a_listed_reporter_is_made_for_every_journey_and_given_its_id() {
    tallied("first");
    tallied("second");
    let tallied = TALLIED.lock().unwrap();
    assert!(tallied.contains(&r#"Some("south-first") 1"#.to_owned()));
    assert!(tallied.contains(&r#"Some("south-second") 1"#.to_owned()));
}

#[workflow]
#[with_step(Ship, name = "ship")]
struct Courier;

#[test]
fn a_workflow_is_named_after_its_type_unless_its_attribute_names_it() {
    assert_eq!(Courier::descriptor().name().to_string(), "Courier");
}

#[test]
fn a_workflow_without_fields_is_created_without_setters() {
    let instance = Courier::builder().create().unwrap();
    let result = LocalExecutor::new().run(instance).unwrap();
    assert!(result.status.is_succeeded());
}

#[test]
fn initial_global_metadata_set_through_the_builder_reaches_the_reporters() {
    let recorder = RecordingReporter::default();
    let instance = Courier::builder()
        .reporters_global_metadata("tenant", "acme".to_string())
        .reporter(recorder.clone())
        .create()
        .unwrap();
    LocalExecutor::new().run(instance).unwrap();
    let metadata = recorder.global_metadata();
    let tenant = metadata
        .get("tenant")
        .and_then(AnyValue::downcast_ref::<String>);
    assert_eq!(tenant, Some(&"acme".to_string()));
}

#[workflow]
#[with_step(Ship, name = "ship")]
struct Ledger {
    first: u32,
    field_0: u32,
}

#[test]
fn a_field_may_have_the_name_of_a_variable_of_the_generated_builder() {
    let instance = Ledger::builder().first(1).field_0(2).create().unwrap();
    let ledger = instance.workflow();
    assert_eq!((ledger.first, ledger.field_0), (1, 2));
}
