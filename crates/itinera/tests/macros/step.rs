//! Steps the macros describe, against the same steps written for the builder, in a synchronous
//! workflow.

use itinera::error::Error;
use itinera::journey::Contributor;
use itinera::step::{
    DescribedStep, Input, OptionalInput, Outcome, Resolved, Step, StepDescriptor, StepFactory,
    StepNeeds, StepReporter, step, step_name,
};
use itinera::workflow::WorkflowDescriptor;
use rstest::rstest;

use super::fixtures::{
    A_GIFT, AN_EMPTY_GIFT_NOTE, NO_AMOUNT, NO_GIFT_WITH_A_COUPON, NOT_SAYING_WHETHER_IT_IS_A_GIFT,
    NOTHING_TO_CHARGE, Order, Orders, orders, run,
};

#[derive(Step)]
struct Charge {
    #[input]
    amount: i64,
    #[input("coupon")]
    discount: Option<String>,
    #[default]
    fees: Vec<i64>,
}

#[step]
impl Charge {
    fn run(
        mut self,
        contributor: &mut Contributor,
        mut reporter: StepReporter<'_>,
    ) -> Result<Outcome, Error> {
        if self.amount <= 0 {
            return Self::fail("nothing-to-charge");
        }
        if self.discount.is_none() {
            self.fees.push(2);
        }
        let total = self.amount + self.fees.iter().sum::<i64>();
        reporter.info_with("charging", total)?;
        contributor.contribute("total", total);
        Self::succeed()
    }
}

struct Ship {
    gift_note: Option<String>,
}

#[step]
impl Ship {
    fn new(#[input(required)] gift_note: Option<String>) -> Result<Self, Error> {
        if gift_note.as_ref().is_some_and(String::is_empty) {
            return Err(Error::msg("a gift note is never empty"));
        }
        Ok(Self { gift_note })
    }

    fn run(self, mut reporter: StepReporter<'_>) -> Result<Outcome, Error> {
        match self.gift_note {
            Some(note) => reporter.info_with("gift note", note)?,
            None => return Self::skip_because("no-gift-note"),
        }
        Self::succeed()
    }
}

const AMOUNT: Input<i64> = Input::new("amount");
const COUPON: OptionalInput<String> = OptionalInput::new("coupon");
const GIFT_NOTE: Input<Option<String>> = Input::new("gift_note");

struct HandCharge<'a> {
    amount: i64,
    discount: Option<String>,
    contributor: &'a mut Contributor,
    reporter: StepReporter<'a>,
}

impl Step for HandCharge<'_> {
    fn run(mut self) -> Result<Outcome, Error> {
        if self.amount <= 0 {
            return Ok(Outcome::failure("nothing-to-charge"));
        }
        let fees = if self.discount.is_none() { 2 } else { 0 };
        let total = self.amount + fees;
        self.reporter.info_with("charging", total)?;
        self.contributor.contribute("total", total);
        Ok(Outcome::success())
    }
}

struct ChargeFactory;

impl StepFactory for ChargeFactory {
    type Step<'a> = HandCharge<'a>;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new()
            .input(&AMOUNT)
            .optional_input(&COUPON)
            .contributor()
            .reporter()
    }

    fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<HandCharge<'a>, Error> {
        Ok(HandCharge {
            amount: got.input(&AMOUNT)?,
            discount: got.optional_input(&COUPON)?,
            contributor: got.contributor()?,
            reporter: got.reporter()?,
        })
    }
}

struct HandShip<'a> {
    gift_note: Option<String>,
    reporter: StepReporter<'a>,
}

impl Step for HandShip<'_> {
    fn run(mut self) -> Result<Outcome, Error> {
        match self.gift_note {
            Some(note) => self.reporter.info_with("gift note", note)?,
            None => return Ok(Outcome::skipped_because("no-gift-note")),
        }
        Ok(Outcome::success())
    }
}

struct ShipFactory;

impl StepFactory for ShipFactory {
    type Step<'a> = HandShip<'a>;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new().input(&GIFT_NOTE).reporter()
    }

    fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<HandShip<'a>, Error> {
        let gift_note = got.input(&GIFT_NOTE)?;
        if gift_note.as_ref().is_some_and(String::is_empty) {
            return Err(Error::msg("a gift note is never empty"));
        }
        Ok(HandShip {
            gift_note,
            reporter: got.reporter()?,
        })
    }
}

fn by_macros() -> WorkflowDescriptor<Orders> {
    orders()
        .step(Charge::descriptor(step_name!("charge")))
        .step(Ship::descriptor(step_name!("ship")))
        .build()
        .unwrap()
}

fn by_hand() -> WorkflowDescriptor<Orders> {
    orders()
        .step(StepDescriptor::new(step_name!("charge"), ChargeFactory))
        .step(StepDescriptor::new(step_name!("ship"), ShipFactory))
        .build()
        .unwrap()
}

#[rstest]
#[case::a_gift(A_GIFT)]
#[case::no_gift_with_a_coupon(NO_GIFT_WITH_A_COUPON)]
#[case::nothing_to_charge(NOTHING_TO_CHARGE)]
#[case::no_amount(NO_AMOUNT)]
#[case::not_saying_whether_it_is_a_gift(NOT_SAYING_WHETHER_IT_IS_A_GIFT)]
#[case::an_empty_gift_note(AN_EMPTY_GIFT_NOTE)]
fn steps_the_macros_describe_list_and_run_as_those_written_for_the_builder(#[case] order: Order) {
    assert_eq!(run(&by_macros(), order), run(&by_hand(), order));
}

#[test]
fn a_step_may_take_the_names_of_the_parts_of_a_step() {
    type M = i64;

    #[derive(Step)]
    struct Attempt {
        #[input]
        amount: M,
    }

    #[step]
    impl Attempt {
        fn run(self) -> Result<Outcome, Error> {
            if self.amount > 0 {
                return Self::succeed();
            }
            Self::skip()
        }
    }

    #[derive(Step)]
    struct Factory;

    #[step]
    impl Factory {
        fn run(self) -> Result<Outcome, Error> {
            Self::succeed()
        }
    }

    let workflow = orders()
        .step(Attempt::descriptor(step_name!("attempt")))
        .step(Factory::descriptor(step_name!("factory")))
        .build();
    assert!(workflow.is_ok());
}
