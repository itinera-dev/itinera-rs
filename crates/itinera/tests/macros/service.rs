//! Steps that request services, and the providers that supply them, declared with the macros,
//! against the same written for the builder, in a synchronous workflow.

use itinera::error::Error;
use itinera::journey::{Contributor, DataBag, InitialData, JourneyId};
use itinera::policy::{HookNeeds, HookReporter, Provider, Requested};
use itinera::report::RecordingReporter;
use itinera::service::{
    PerAttempt, PerJourney, Provided, Service, ServiceProviderDescriptor, provide,
};
use itinera::step::{
    Attempt, Input, OptionalInput, Outcome, Resolved, Step, StepDescriptor, StepFactory, StepName,
    StepNeeds, step, step_name,
};
use itinera::workflow::{Workflow, WorkflowDescriptor, workflow};
use rstest::rstest;

use super::fixtures::{Journey, run_created};

/// Charges amounts, up to a limit.
trait PaymentGateway: Send + Sync {
    fn charge(&self, amount: i64) -> Option<String>;
}

/// Reserves the goods of an order.
trait StockStore: Send + Sync {
    fn reserve(&self, amount: i64) -> Option<String>;
}

struct Gateway {
    name: String,
    limit: i64,
}

impl PaymentGateway for Gateway {
    fn charge(&self, amount: i64) -> Option<String> {
        (amount <= self.limit).then(|| format!("{amount} through {}", self.name))
    }
}

struct Warehouse {
    journey: String,
}

impl StockStore for Warehouse {
    fn reserve(&self, amount: i64) -> Option<String> {
        (amount < 100).then(|| format!("{amount} reserved for {}", self.journey))
    }
}

/// The amount, once the stock is reserved for it.
fn reserved(amount: i64, stock: &dyn StockStore) -> Result<i64, Error> {
    match stock.reserve(amount) {
        Some(_) => Ok(amount),
        None => Err(Error::msg("out of stock")),
    }
}

/// Charges the amount, failing when the gateway refuses it.
fn charged(
    amount: i64,
    gateway: &dyn PaymentGateway,
    stock: &dyn StockStore,
    contributor: &mut Contributor,
) -> Outcome {
    match (gateway.charge(amount), stock.reserve(amount)) {
        (Some(receipt), Some(reservation)) => {
            contributor.contribute("receipt", receipt);
            contributor.contribute("reservation", reservation);
            Outcome::success()
        }
        _ => Outcome::failure("refused"),
    }
}

fn shipped(stock: &dyn StockStore, contributor: &mut Contributor) -> Outcome {
    contributor.contribute("shipped", stock.reserve(1));
    Outcome::success()
}

/// The stock of the journey.
fn stocked(journey: &JourneyId) -> Provided<dyn StockStore> {
    provide!(Warehouse {
        journey: journey.to_string(),
    })
}

/// The workflow's gateway on a first attempt, which charges up to 50, and a backup on later ones.
fn gateway(
    name: &str,
    step: StepName,
    attempt: Attempt,
    currency: Option<String>,
    mut reporter: HookReporter<'_>,
) -> Result<Provided<dyn PaymentGateway>, Error> {
    reporter.info(format!("{step} attempt {attempt}"))?;
    let currency = currency.unwrap_or_default();
    let (name, limit) = if attempt.is_first() {
        (format!("{name} {currency}"), 50)
    } else {
        (format!("backup {currency}"), i64::MAX)
    };
    Ok(provide!(Gateway { name, limit }))
}

fn first_order(_: &DataBag) -> String {
    "order-1".to_owned()
}

struct Charge {
    amount: i64,
}

#[step]
impl Charge {
    fn new(#[input] amount: i64, #[service] stock: &dyn StockStore) -> Result<Self, Error> {
        Ok(Self {
            amount: reserved(amount, stock)?,
        })
    }

    fn run(
        self,
        #[service("payments")] gateway: &dyn PaymentGateway,
        #[service] stock: &dyn StockStore,
        contributor: &mut Contributor,
    ) -> Result<Outcome, Error> {
        Ok(charged(self.amount, gateway, stock, contributor))
    }
}

#[derive(Step)]
struct Ship;

#[step]
impl Ship {
    fn run(
        self,
        #[service] stock: &dyn StockStore,
        contributor: &mut Contributor,
    ) -> Result<Outcome, Error> {
        Ok(shipped(stock, contributor))
    }
}

#[workflow(name = "orders")]
#[with_step(Charge, name = "charge", retry_budget = 1)]
#[with_step(Ship, name = "ship")]
struct Shop {
    gateway: &'static str,
}

#[workflow]
impl Shop {
    #[provider]
    fn stock(&self, journey: &JourneyId) -> Result<Provided<dyn StockStore>, Error> {
        Ok(stocked(journey))
    }

    #[provider(per_attempt, steps = ["charge"], key = "payments", name = "charging")]
    fn payments(
        &self,
        step: StepName,
        attempt: Attempt,
        #[from_workflow] currency: Option<String>,
        reporter: HookReporter<'_>,
    ) -> Result<Provided<dyn PaymentGateway>, Error> {
        gateway(self.gateway, step, attempt, currency, reporter)
    }

    #[id_generator]
    fn order_id(&self, data: &DataBag) -> Result<String, Error> {
        Ok(first_order(data))
    }
}

/// The same shop, declared with the builder.
struct HandShop {
    gateway: &'static str,
}

const AMOUNT: Input<i64> = Input::new("amount");
const CURRENCY: OptionalInput<String> = OptionalInput::new("currency");
const PAYMENTS: Service<dyn PaymentGateway> = Service::new("payments");
const STOCK: Service<dyn StockStore> = Service::new("stock");

struct HandCharge<'a> {
    amount: i64,
    gateway: &'a dyn PaymentGateway,
    stock: &'a dyn StockStore,
    contributor: &'a mut Contributor,
}

impl Step for HandCharge<'_> {
    fn run(self) -> Result<Outcome, Error> {
        Ok(charged(
            self.amount,
            self.gateway,
            self.stock,
            self.contributor,
        ))
    }
}

struct ChargeFactory;

impl StepFactory for ChargeFactory {
    type Step<'a> = HandCharge<'a>;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new()
            .input(&AMOUNT)
            .service(&STOCK)
            .contributor()
            .service(&PAYMENTS)
    }

    fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<HandCharge<'a>, Error> {
        let stock = got.service(&STOCK)?;
        Ok(HandCharge {
            amount: reserved(got.input(&AMOUNT)?, stock)?,
            gateway: got.service(&PAYMENTS)?,
            stock,
            contributor: got.contributor()?,
        })
    }
}

struct HandShip<'a> {
    stock: &'a dyn StockStore,
    contributor: &'a mut Contributor,
}

impl Step for HandShip<'_> {
    fn run(self) -> Result<Outcome, Error> {
        Ok(shipped(self.stock, self.contributor))
    }
}

struct ShipFactory;

impl StepFactory for ShipFactory {
    type Step<'a> = HandShip<'a>;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new().contributor().service(&STOCK)
    }

    fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<HandShip<'a>, Error> {
        Ok(HandShip {
            stock: got.service(&STOCK)?,
            contributor: got.contributor()?,
        })
    }
}

fn hand_stock(
    _: &HandShop,
    got: Requested<'_, HandShop, Provider<PerJourney>>,
) -> Result<Provided<dyn StockStore>, Error> {
    Ok(stocked(got.journey_id()))
}

fn hand_payments(
    shop: &HandShop,
    mut got: Requested<'_, HandShop, Provider<PerAttempt>>,
) -> Result<Provided<dyn PaymentGateway>, Error> {
    let currency = got.optional_from_workflow(&CURRENCY)?;
    let reporter = got.reporter()?;
    gateway(
        shop.gateway,
        got.step_name(),
        got.attempt(),
        currency,
        reporter,
    )
}

fn hand_order_id(_: &HandShop, data: &DataBag) -> Result<String, Error> {
    Ok(first_order(data))
}

fn by_hand() -> WorkflowDescriptor<HandShop> {
    let charging = ServiceProviderDescriptor::per_attempt("charging", &PAYMENTS, hand_payments)
        .needing(
            HookNeeds::new()
                .optional_from_workflow(&CURRENCY)
                .reporter(),
        )
        .step(step_name!("charge"));
    WorkflowDescriptor::builder("orders")
        .step(StepDescriptor::new(step_name!("charge"), ChargeFactory).retry_budget(1))
        .step(StepDescriptor::new(step_name!("ship"), ShipFactory))
        .provider(ServiceProviderDescriptor::new("stock", &STOCK, hand_stock))
        .provider(charging)
        .id_generator(hand_order_id)
        .build()
        .unwrap()
}

/// The initial data of a journey of the shop.
#[derive(Clone, InitialData)]
struct NewOrder {
    amount: i64,
    currency: Option<String>,
}

fn by_macros(order: NewOrder) -> Journey {
    run_created(Shop::descriptor(), |recorder| {
        Shop::builder()
            .gateway("main")
            .initial_data(order)
            .reporter(recorder)
            .create()
            .unwrap()
    })
}

fn by_the_builder(order: NewOrder) -> Journey {
    let workflow = by_hand();
    run_created(&workflow, |recorder| {
        workflow
            .instance(HandShop { gateway: "main" })
            .initial_data(order)
            .reporter(recorder)
            .create()
            .unwrap()
    })
}

fn order(amount: i64, currency: Option<&str>) -> NewOrder {
    NewOrder {
        amount,
        currency: currency.map(str::to_owned),
    }
}

#[rstest]
#[case::charged_on_the_first_attempt(order(40, Some("EUR")))]
#[case::charged_by_the_backup_on_a_retry(order(70, None))]
#[case::out_of_stock(order(120, Some("EUR")))]
fn services_the_macros_request_and_provide_list_and_run_as_those_written_for_the_builder(
    #[case] order: NewOrder,
) {
    assert_eq!(by_macros(order.clone()), by_the_builder(order));
}

/// A gateway that charges any amount at once, for tests.
struct Unlimited;

impl PaymentGateway for Unlimited {
    fn charge(&self, amount: i64) -> Option<String> {
        Some(format!("{amount} without a limit"))
    }
}

fn unlimited() -> Result<Provided<dyn PaymentGateway>, Error> {
    Ok(provide!(Unlimited))
}

#[test]
fn a_fake_given_to_the_builder_the_macros_generate_runs_as_one_given_to_the_instance_builder() {
    let paths = RecordingReporter::default();
    let faked_by_macros = run_created(Shop::descriptor(), |recorder| {
        Shop::builder()
            .gateway("main")
            .initial_data(order(70, None))
            .reporter(recorder)
            .reporter(paths.clone())
            .fake_provider::<dyn PaymentGateway>("charging", unlimited)
            .create()
            .unwrap()
    });
    let workflow = by_hand();
    let faked_by_hand = run_created(&workflow, |recorder| {
        workflow
            .instance(HandShop { gateway: "main" })
            .initial_data(order(70, None))
            .reporter(recorder)
            .fake_provider::<dyn PaymentGateway>("charging", unlimited)
            .create()
            .unwrap()
    });

    assert_eq!(faked_by_macros, faked_by_hand);
    assert_eq!(
        paths.path(),
        [
            "charge attempt 1 succeeded",
            "ship attempt 1 succeeded",
            "journey succeeded"
        ]
    );
}

#[derive(Step)]
struct Restock;

#[step]
impl Restock {
    fn run(self, #[service("stock")] stock: &dyn StockStore) -> Result<Outcome, Error> {
        match stock.reserve(1) {
            Some(_) => Ok(Outcome::success()),
            None => Ok(Outcome::failure("out-of-stock")),
        }
    }
}

#[workflow]
#[with_step(Restock, name = "restock")]
struct Depot;

#[workflow]
impl Depot {
    #[provider]
    fn stock(&self, journey: &JourneyId) -> Result<Provided<dyn StockStore>, Error> {
        Ok(stocked(journey))
    }

    #[provider(key = "stock", steps = ["restock"])]
    fn shelf(&self, journey: &JourneyId) -> Result<Provided<Warehouse>, Error> {
        Ok(provide!(Warehouse {
            journey: journey.to_string(),
        }))
    }
}

#[test]
#[should_panic(expected = "wrong service type: shelf for stock of restock")]
fn a_service_whose_providers_return_different_types_is_checked_when_described() {
    Depot::descriptor();
}
