use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::instance::{InstanceBuilder, WorkflowInstance};
use itinera::journey::{DataBag, InitialData, JourneyResult, Output, OutputError};
use itinera::step::{DescribedStep, Outcome, step, step_name};
use rstest::rstest;

use crate::fixtures::{Orders, orders, written};

#[derive(Clone, InitialData)]
struct NewOrder {
    price: i64,
    #[key("discount_code")]
    code: Option<String>,
    #[required]
    gift_note: Option<String>,
}

impl NewOrder {
    /// The same data, given key by key.
    fn given_to(self, builder: InstanceBuilder<Orders>) -> InstanceBuilder<Orders> {
        let builder = builder
            .data("price", self.price)
            .data("gift_note", self.gift_note);
        match self.code {
            Some(code) => builder.data("discount_code", code),
            None => builder,
        }
    }
}

const WITHOUT_OPTIONAL_KEYS: NewOrder = NewOrder {
    price: 42,
    code: None,
    gift_note: None,
};

fn every_key() -> NewOrder {
    NewOrder {
        code: Some("spring".to_owned()),
        gift_note: Some("happy birthday".to_owned()),
        ..WITHOUT_OPTIONAL_KEYS
    }
}

fn data(builder: InstanceBuilder<Orders>) -> Vec<String> {
    builder
        .create()
        .unwrap()
        .data_bag()
        .into_iter()
        .map(written)
        .collect()
}

#[rstest]
#[case::every_key(every_key())]
#[case::without_optional_keys(WITHOUT_OPTIONAL_KEYS)]
fn initial_data_the_derive_gives_is_the_data_given_key_by_key(#[case] order: NewOrder) {
    let orders = orders().build().unwrap();
    assert_eq!(
        data(orders.instance(Orders).initial_data(order.clone())),
        data(order.given_to(orders.instance(Orders))),
    );
}

#[derive(Debug, PartialEq, Output)]
struct Receipt {
    price: i64,
    #[key("discount_code")]
    code: Option<String>,
    #[required]
    gift_note: Option<String>,
}

/// The same output, read key by key.
struct ReadByHand {
    receipt: Receipt,
}

impl Output for ReadByHand {
    fn from_data(data: &DataBag) -> Result<Self, OutputError> {
        let receipt = Receipt {
            price: data.required("price")?,
            code: data.optional("discount_code")?,
            gift_note: data.required("gift_note")?,
        };
        Ok(Self { receipt })
    }
}

struct Closed;

#[step]
impl Closed {
    fn new() -> Result<Self, Error> {
        Err(Error::msg("the shop is closed"))
    }

    fn run(self) -> Result<Outcome, Error> {
        Self::succeed()
    }
}

fn run(given: impl FnOnce(InstanceBuilder<Orders>) -> InstanceBuilder<Orders>) -> JourneyResult {
    let orders = orders().build().unwrap();
    let instance = given(orders.instance(Orders)).create().unwrap();
    LocalExecutor::new().run(instance).unwrap()
}

fn with_every_key() -> JourneyResult {
    run(|builder| every_key().given_to(builder))
}

fn without_a_discount_code() -> JourneyResult {
    run(|builder| WITHOUT_OPTIONAL_KEYS.given_to(builder))
}

fn without_a_gift_note() -> JourneyResult {
    run(|builder| builder.data("price", 42_i64))
}

fn without_a_price() -> JourneyResult {
    run(|builder| builder.data("gift_note", None::<String>))
}

fn priced_in_words() -> JourneyResult {
    run(|builder| {
        WITHOUT_OPTIONAL_KEYS
            .given_to(builder)
            .data("price", "forty-two".to_owned())
    })
}

fn aborted() -> JourneyResult {
    let orders = orders()
        .step(Closed::descriptor(step_name!("close")))
        .build()
        .unwrap();
    let instance = orders.instance(Orders).create().unwrap();
    LocalExecutor::new().run(instance).unwrap()
}

fn receipt(read: ReadByHand) -> Receipt {
    read.receipt
}

#[rstest]
#[case::with_every_key(with_every_key)]
#[case::without_a_discount_code(without_a_discount_code)]
#[case::without_a_gift_note(without_a_gift_note)]
#[case::without_a_price(without_a_price)]
#[case::priced_in_words(priced_in_words)]
#[case::aborted(aborted)]
fn an_output_the_derive_reads_is_what_reading_key_by_key_gives(
    #[case] journey: fn() -> JourneyResult,
) {
    let result = journey();
    assert_eq!(
        result.output::<Receipt>(),
        result.output::<ReadByHand>().map(receipt),
    );
}

#[test]
fn a_required_option_is_missing_when_the_data_bag_does_not_hold_its_key() {
    let missing = OutputError::Missing {
        key: "gift_note".to_owned(),
    };
    assert_eq!(without_a_gift_note().output::<Receipt>(), Err(missing));
}

#[derive(Debug, PartialEq, InitialData, Output)]
struct Nothing;

#[test]
fn a_struct_without_fields_gives_and_reads_no_key() {
    let orders = orders().build().unwrap();
    assert!(data(orders.instance(Orders).initial_data(Nothing)).is_empty());
    assert_eq!(with_every_key().output::<Nothing>(), Ok(Nothing));
}
