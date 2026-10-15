//! Steps that request services, and the providers that supply them, declared with the macros,
//! against the same written for the builder, in an asynchronous workflow.

use itinera::error::Error;
use itinera::journey::{Contributor, DataBag, InitialData};
use itinera::mode::Asynchronous;
use itinera::policy::{AsyncHookReporter, HookNeeds, Provider, Requested};
use itinera::report::RecordingReporter;
use itinera::service::{
    PerAttempt, PerJourney, Provided, Service, ServiceProviderDescriptor, provide,
};
use itinera::step::{
    AsyncStep, AsyncStepFactory, Attempt, Outcome, Resolved, StepDescriptor, StepNeeds, step,
    step_name,
};
use itinera::workflow::{Workflow, WorkflowDescriptor, workflow};
use rstest::rstest;

use super::fixtures::{Journey, run_created_async};

/// Dispatches parcels, to a limited number of places.
trait Courier: Send + Sync {
    fn dispatch(&self, parcel: &str) -> Option<String>;
}

/// Labels the parcels of a journey.
trait Labeller: Send + Sync {
    fn label(&self, parcel: &str) -> String;
}

struct Van {
    name: String,
    reach: usize,
}

impl Courier for Van {
    fn dispatch(&self, parcel: &str) -> Option<String> {
        (parcel.len() <= self.reach).then(|| format!("{parcel} by {}", self.name))
    }
}

struct Printer;

impl Labeller for Printer {
    fn label(&self, parcel: &str) -> String {
        format!("label for {parcel}")
    }
}

/// Dispatches the parcel, failing when the courier cannot reach it.
fn dispatched(
    parcel: &str,
    courier: &dyn Courier,
    labeller: &dyn Labeller,
    contributor: &mut Contributor,
) -> Outcome {
    match courier.dispatch(parcel) {
        Some(dispatch) => {
            contributor.contribute("dispatch", dispatch);
            contributor.contribute("label", labeller.label(parcel));
            Outcome::success()
        }
        None => Outcome::failure("unreachable"),
    }
}

/// A van that reaches parcels of up to five letters on a first attempt, and a lorry that
/// reaches every parcel on later ones.
async fn courier(
    attempt: Attempt,
    mut reporter: AsyncHookReporter<'_>,
) -> Result<Provided<dyn Courier>, Error> {
    reporter.info(format!("attempt {attempt}")).await?;
    let (name, reach) = if attempt.is_first() {
        ("van", 5)
    } else {
        ("lorry", usize::MAX)
    };
    Ok(provide!(Van {
        name: name.to_owned(),
        reach,
    }))
}

fn first_parcel(_: &DataBag) -> String {
    "parcel-1".to_owned()
}

struct Dispatch {
    parcel: String,
}

#[step]
impl Dispatch {
    fn new(#[input] parcel: String) -> Self {
        Self { parcel }
    }

    async fn run(
        self,
        #[service] courier: &dyn Courier,
        #[service] labeller: &dyn Labeller,
        contributor: &mut Contributor,
    ) -> Result<Outcome, Error> {
        Ok(dispatched(&self.parcel, courier, labeller, contributor))
    }
}

#[workflow(name = "parcels")]
#[with_step(Dispatch, name = "dispatch", retry_budget = 1)]
struct Parcels;

#[workflow]
impl Parcels {
    #[provider]
    fn labeller(&self) -> Result<Provided<dyn Labeller>, Error> {
        Ok(provide!(Printer))
    }

    #[provider(per_attempt)]
    async fn courier(
        &self,
        attempt: Attempt,
        reporter: AsyncHookReporter<'_>,
    ) -> Result<Provided<dyn Courier>, Error> {
        courier(attempt, reporter).await
    }

    #[id_generator]
    fn parcel_id(&self, data: &DataBag) -> Result<String, Error> {
        Ok(first_parcel(data))
    }
}

/// The same workflow, declared with the builder.
struct HandParcels;

const PARCEL: itinera::step::Input<String> = itinera::step::Input::new("parcel");
const COURIER: Service<dyn Courier> = Service::new("courier");
const LABELLER: Service<dyn Labeller> = Service::new("labeller");

struct HandDispatch<'a> {
    parcel: String,
    courier: &'a dyn Courier,
    labeller: &'a dyn Labeller,
    contributor: &'a mut Contributor,
}

impl AsyncStep for HandDispatch<'_> {
    async fn run(self) -> Result<Outcome, Error> {
        Ok(dispatched(
            &self.parcel,
            self.courier,
            self.labeller,
            self.contributor,
        ))
    }
}

struct DispatchFactory;

impl AsyncStepFactory for DispatchFactory {
    type Step<'a> = HandDispatch<'a>;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new()
            .input(&PARCEL)
            .contributor()
            .service(&COURIER)
            .service(&LABELLER)
    }

    fn build<'a>(
        &'a self,
        got: &mut Resolved<'a, Asynchronous>,
    ) -> Result<HandDispatch<'a>, Error> {
        Ok(HandDispatch {
            parcel: got.input(&PARCEL)?,
            courier: got.service(&COURIER)?,
            labeller: got.service(&LABELLER)?,
            contributor: got.contributor()?,
        })
    }
}

async fn hand_labeller(
    _: &HandParcels,
    _: Requested<'_, HandParcels, Provider<PerJourney>, Asynchronous>,
) -> Result<Provided<dyn Labeller>, Error> {
    Ok(provide!(Printer))
}

async fn hand_courier(
    _: &HandParcels,
    mut got: Requested<'_, HandParcels, Provider<PerAttempt>, Asynchronous>,
) -> Result<Provided<dyn Courier>, Error> {
    let reporter = got.reporter()?;
    courier(got.attempt(), reporter).await
}

fn hand_parcel_id(_: &HandParcels, data: &DataBag) -> Result<String, Error> {
    Ok(first_parcel(data))
}

fn by_hand() -> WorkflowDescriptor<HandParcels, Asynchronous> {
    let courier = ServiceProviderDescriptor::per_attempt_async("courier", &COURIER, hand_courier)
        .needing(HookNeeds::new().reporter());
    WorkflowDescriptor::async_builder("parcels")
        .step(StepDescriptor::new_async(step_name!("dispatch"), DispatchFactory).retry_budget(1))
        .provider(ServiceProviderDescriptor::new_async(
            "labeller",
            &LABELLER,
            hand_labeller,
        ))
        .provider(courier)
        .id_generator(hand_parcel_id)
        .build()
        .unwrap()
}

/// The initial data of a journey of parcels.
#[derive(Clone, InitialData)]
struct NewParcel {
    parcel: String,
}

fn by_macros(parcel: NewParcel) -> Journey {
    run_created_async(Parcels::descriptor(), |recorder| {
        Parcels::builder()
            .initial_data(parcel)
            .reporter(recorder)
            .create()
            .unwrap()
    })
}

fn by_the_builder(parcel: NewParcel) -> Journey {
    let workflow = by_hand();
    run_created_async(&workflow, |recorder| {
        workflow
            .instance(HandParcels)
            .initial_data(parcel)
            .reporter(recorder)
            .create()
            .unwrap()
    })
}

fn parcel(name: &str) -> NewParcel {
    NewParcel {
        parcel: name.to_owned(),
    }
}

#[rstest]
#[case::dispatched_on_the_first_attempt(parcel("books"))]
#[case::dispatched_by_the_lorry_on_a_retry(parcel("a wardrobe"))]
fn services_the_macros_request_and_provide_list_and_run_as_those_written_for_the_builder(
    #[case] parcel: NewParcel,
) {
    assert_eq!(by_macros(parcel.clone()), by_the_builder(parcel));
}

/// A courier that reaches every parcel at once, for tests.
fn drone() -> Result<Provided<dyn Courier>, Error> {
    Ok(provide!(Van {
        name: "drone".to_owned(),
        reach: usize::MAX,
    }))
}

#[test]
fn a_fake_serves_an_asynchronous_workflow_alike_through_either_builder() {
    let paths = RecordingReporter::default();
    let faked_by_macros = run_created_async(Parcels::descriptor(), |recorder| {
        Parcels::builder()
            .initial_data(parcel("a wardrobe"))
            .reporter(recorder)
            .reporter(paths.clone())
            .fake_provider::<dyn Courier>("courier", drone)
            .create()
            .unwrap()
    });
    let workflow = by_hand();
    let faked_by_hand = run_created_async(&workflow, |recorder| {
        workflow
            .instance(HandParcels)
            .initial_data(parcel("a wardrobe"))
            .reporter(recorder)
            .fake_provider::<dyn Courier>("courier", drone)
            .create()
            .unwrap()
    });

    assert_eq!(faked_by_macros, faked_by_hand);
    assert_eq!(
        paths.path(),
        ["dispatch attempt 1 succeeded", "journey succeeded"]
    );
}
