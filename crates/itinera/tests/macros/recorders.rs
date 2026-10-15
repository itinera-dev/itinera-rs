//! Steps, hooks and input adapters the macros declare, called directly by a test with the
//! handles the recorders lend, outside any journey.

use itinera::error::Error;
use itinera::event::Level;
use itinera::journey::{Contributions, Contributor, DataBag, Read};
use itinera::policy::{HookEventReporter, HookReporter, OnSuccess, step_policy};
use itinera::service::{Provided, provide};
use itinera::step::{Outcome, Reason, Step, StepEventReporter, StepReporter, step};
use itinera::workflow::{Supplied, workflow};

/// Takes payments.
trait PaymentGateway: Send + Sync {
    fn take(&self, amount: i64) -> Option<String>;
}

/// A gateway that takes any amount up to its limit.
struct FakeGateway {
    limit: i64,
}

impl PaymentGateway for FakeGateway {
    fn take(&self, amount: i64) -> Option<String> {
        (amount <= self.limit).then(|| format!("P-{amount}"))
    }
}

#[derive(Step)]
struct Charge {
    #[input]
    price: i64,
}

#[step]
impl Charge {
    fn run(
        self,
        #[service] payments: &dyn PaymentGateway,
        #[contributor] contributor: &mut Contributor,
        #[reporter] mut reporter: StepReporter<'_>,
    ) -> Result<Outcome, Error> {
        reporter.info_with("charging", self.price)?;
        let Some(payment) = payments.take(self.price) else {
            return Self::fail(Reason::new("declined"));
        };
        reporter.set_global_metadata("payment", payment.clone())?;
        contributor.contribute("payment", payment);
        Self::succeed()
    }
}

#[derive(Default)]
struct Audit;

#[step_policy(name = "audit")]
impl Audit {
    #[on_step_success]
    fn audited(
        &self,
        #[from_step] payment: String,
        #[contributor] contributor: &mut Contributor,
        #[reporter] mut reporter: HookReporter<'_>,
    ) -> Result<OnSuccess, Error> {
        reporter.warning_with("audited", payment.clone())?;
        contributor.contribute("audit", format!("paid {payment}"));
        Self::keep_default()
    }
}

#[workflow]
#[with_step(Charge, name = "charge")]
struct Shop {
    fee: i64,
}

#[workflow]
impl Shop {
    #[input_adapter(steps = ["charge"])]
    fn pricing(&self, #[key] key: &str, #[data_bag] data: &DataBag) -> Result<Supplied, Error> {
        match data.read::<i64>(key) {
            Read::Present(price) => Self::supply(price + self.fee),
            Read::Absent | Read::OtherType => Self::from_data_bag(),
        }
    }

    #[provider(per_attempt, steps = ["charge"])]
    fn payments(&self) -> Result<Provided<dyn PaymentGateway>, Error> {
        Ok(provide!(FakeGateway { limit: 50 }))
    }
}

/// Runs the charge for a price, through a gateway that takes up to 50, with the recorders' handles.
fn charge(
    price: i64,
    contributions: &mut Contributions,
    events: &mut StepEventReporter,
) -> Result<Outcome, Error> {
    Charge { price }.run(
        &FakeGateway { limit: 50 },
        contributions.contributor(),
        events.reporter(),
    )
}

#[test]
fn a_steps_run_called_with_recorded_handles_leaves_its_contributions_and_events_to_read() {
    let mut contributions = Contributions::default();
    let mut events = StepEventReporter::default();

    let outcome = charge(40, &mut contributions, &mut events).unwrap();

    assert!(outcome.is_success());
    assert_eq!(
        contributions.read::<String>("payment"),
        Read::Present("P-40".to_string())
    );
    let charging = events.emitted().first().unwrap();
    assert_eq!(charging.level(), Level::Info);
    assert_eq!(charging.data::<i64>(), Some(&40));
    assert!(events.global_metadata().get("payment").is_some());
}

#[test]
fn a_reporter_failure_reaches_the_test_through_the_steps_question_mark() {
    let mut contributions = Contributions::default();
    let mut events = StepEventReporter::failing();

    let ran = charge(40, &mut contributions, &mut events);

    assert!(ran.is_err());
    assert_eq!(contributions.keys().count(), 0);
}

#[test]
fn a_hook_called_with_recorded_handles_leaves_its_contributions_and_events_to_read() {
    let mut contributions = Contributions::default();
    let mut events = HookEventReporter::default();

    let decided = Audit
        .audited(
            "P-40".to_string(),
            contributions.contributor(),
            events.reporter(),
        )
        .unwrap();

    assert!(decided.keeps_default());
    assert_eq!(
        contributions.read::<String>("audit"),
        Read::Present("paid P-40".to_string())
    );
    assert_eq!(events.emitted().first().unwrap().message(), "audited");
}

#[test]
fn an_input_adapter_reads_a_data_bag_a_test_builds_from_values() {
    let data = DataBag::default().with("price", 40_i64);
    let shop = Shop { fee: 2 };

    let supplied = shop.pricing("price", &data).unwrap();
    let left = shop.pricing("coupon", &data).unwrap();

    assert!(matches!(supplied, Supplied::Value(value) if value.downcast_ref::<i64>() == Some(&42)));
    assert!(matches!(left, Supplied::FromDataBag));
}
