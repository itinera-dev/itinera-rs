//! An asynchronous workflow the macros declare, against the same workflow declared with the
//! builder.

use std::sync::Mutex;

use itinera::error::Error;
use itinera::event::Event;
use itinera::executor::AsyncLocalExecutor;
use itinera::journey::{Contributor, InitialData};
use itinera::mode::Asynchronous;
use itinera::policy::{OnSuccess, StepPolicy, WorkflowPolicy, step_policy, workflow_policy};
use itinera::report::{AsyncReporter, GlobalMetadata};
use itinera::step::{DescribedStep, Outcome, Step, step, step_name};
use itinera::value::AnyValue;
use itinera::workflow::{Workflow, WorkflowDescriptor, workflow};
use rstest::rstest;

use super::fixtures::{Journey, run_created_async};

/// Dispatches the parcel the initial data names.
#[derive(Step)]
struct Dispatch {
    #[input]
    parcel: String,
}

#[step]
impl Dispatch {
    async fn run(self, contributor: &mut Contributor) -> Result<Outcome, Error> {
        if self.parcel.is_empty() {
            return Self::fail("no-parcel");
        }
        contributor.contribute("dispatched", self.parcel);
        Self::succeed()
    }
}

#[derive(Default)]
struct Track;

#[step_policy(name = "track")]
impl Track {
    #[on_step_success]
    async fn tracked(
        &self,
        #[from_step] dispatched: String,
        contributor: &mut Contributor,
    ) -> Result<OnSuccess, Error> {
        contributor.contribute("tracking", format!("track {dispatched}"));
        Self::keep_default()
    }
}

#[derive(Default)]
struct Sign;

#[workflow_policy(name = "sign")]
impl Sign {
    #[on_workflow_failure]
    async fn unsigned(&self, contributor: &mut Contributor) -> Result<(), Error> {
        contributor.contribute("signed", false);
        Ok(())
    }
}

/// The journeys a listed asynchronous reporter saw, with the number of each event.
static LOGGED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// A listed asynchronous reporter, made for every journey.
#[derive(Default)]
struct Log {
    metadata: GlobalMetadata,
}

impl AsyncReporter for Log {
    fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<(), Error> {
        self.metadata.insert(key, value.clone());
        Ok(())
    }

    async fn report(&mut self, event: &Event) -> Result<(), Error> {
        let journey_id = self
            .metadata
            .get(GlobalMetadata::JOURNEY_ID)
            .and_then(AnyValue::downcast_ref::<String>);
        let seen = format!("{journey_id:?} {}", event.sequence);
        LOGGED.lock().unwrap().push(seen);
        Ok(())
    }
}

#[workflow(name = "delivery", policies = [Sign], async_reporters = [Log])]
#[with_step(Dispatch, name = "dispatch", retry_budget = 1, policies = [Track])]
struct Delivery {
    van: &'static str,
}

#[workflow]
impl Delivery {
    #[id_generator]
    fn delivery_id(&self) -> Result<String, Error> {
        Ok(format!("{}-1", self.van))
    }
}

/// The same delivery, declared with the builder.
struct HandDelivery {
    van: &'static str,
}

fn hand_delivery_id(
    delivery: &HandDelivery,
    _: &itinera::journey::DataBag,
) -> Result<String, Error> {
    Ok(format!("{}-1", delivery.van))
}

fn by_hand() -> WorkflowDescriptor<HandDelivery, Asynchronous> {
    WorkflowDescriptor::async_builder("delivery")
        .step(
            Dispatch::descriptor(step_name!("dispatch"))
                .retry_budget(1)
                .policy(Track::descriptor()),
        )
        .policy(Sign::descriptor())
        .async_reporter::<Log>()
        .id_generator(hand_delivery_id)
        .build()
        .unwrap()
}

#[derive(Clone, InitialData)]
struct Parcel {
    parcel: String,
}

fn by_macros(parcel: Parcel) -> Journey {
    run_created_async(Delivery::descriptor(), |recorder| {
        Delivery::builder()
            .van("blue van")
            .initial_data(parcel)
            .reporter(recorder)
            .create()
            .unwrap()
    })
}

fn by_the_builder(parcel: Parcel) -> Journey {
    let workflow = by_hand();
    run_created_async(&workflow, |recorder| {
        workflow
            .instance(HandDelivery { van: "blue van" })
            .initial_data(parcel)
            .reporter(recorder)
            .create()
            .unwrap()
    })
}

#[rstest]
#[case::a_parcel("books")]
#[case::no_parcel("")]
fn an_asynchronous_workflow_the_macros_declare_lists_and_runs_as_one_declared_with_the_builder(
    #[case] parcel: &str,
) {
    let parcel = Parcel {
        parcel: parcel.to_owned(),
    };
    assert_eq!(by_macros(parcel.clone()), by_the_builder(parcel));
}

/// An asynchronous reporter made for one journey, which notes the journey it was given.
struct Witness {
    seen: &'static Mutex<Vec<String>>,
}

impl AsyncReporter for Witness {
    fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<(), Error> {
        if let Some(value) = value.downcast_ref::<String>() {
            self.seen.lock().unwrap().push(format!("{key} {value}"));
        }
        Ok(())
    }

    async fn report(&mut self, _event: &Event) -> Result<(), Error> {
        Ok(())
    }
}

/// Runs one journey of the van, with a reporter made for it.
fn delivered(van: &'static str, seen: &'static Mutex<Vec<String>>) {
    let instance = Delivery::builder()
        .van(van)
        .initial_data(Parcel {
            parcel: "letters".to_owned(),
        })
        .async_reporter(Witness { seen })
        .create()
        .unwrap();
    let result = futures::executor::block_on(AsyncLocalExecutor::new().run(instance)).unwrap();
    assert!(result.status.is_succeeded());
}

#[test]
fn an_asynchronous_reporter_made_for_one_journey_is_given_its_id() {
    static SEEN: Mutex<Vec<String>> = Mutex::new(Vec::new());
    delivered("red van", &SEEN);
    assert_eq!(*SEEN.lock().unwrap(), ["journey_id red van-1"]);
}

#[test]
fn a_listed_asynchronous_reporter_is_made_for_every_journey_and_given_its_id() {
    static SEEN: Mutex<Vec<String>> = Mutex::new(Vec::new());
    delivered("green van", &SEEN);
    delivered("white van", &SEEN);
    assert_eq!(
        *SEEN.lock().unwrap(),
        ["journey_id green van-1", "journey_id white van-1"]
    );
    let logged = LOGGED.lock().unwrap();
    assert!(logged.contains(&r#"Some("green van-1") 1"#.to_owned()));
    assert!(logged.contains(&r#"Some("white van-1") 1"#.to_owned()));
}
