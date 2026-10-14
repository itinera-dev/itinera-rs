//! The courier workflow: packs a parcel and dispatches it, asynchronously, with the labels and
//! the courier its service providers supply.

use std::num::NonZeroU32;

use itinera::error::Error;
use itinera::instance::{Instance, InstanceError};
use itinera::journey::{Contributor, DataBag, InitialData, JourneyId};
use itinera::mode::Asynchronous;
use itinera::policy::AsyncHookReporter;
use itinera::service::{Provided, provide};
use itinera::step::{Outcome, Step, step};
use itinera::workflow::workflow;

/// Labels the parcels of a journey.
trait Labels: Send + Sync {
    fn label(&self, parcel: &str) -> String;
}

/// Carries parcels, as far as it reaches.
trait Courier: Send + Sync {
    fn carry(&self, parcel: &str) -> Option<String>;
}

struct Printer {
    journey_id: String,
}

impl Labels for Printer {
    fn label(&self, parcel: &str) -> String {
        format!("{parcel} for {}", self.journey_id)
    }
}

struct Vehicle {
    name: String,
    reach: usize,
}

impl Courier for Vehicle {
    fn carry(&self, parcel: &str) -> Option<String> {
        (parcel.len() <= self.reach).then(|| format!("{parcel} by {}", self.name))
    }
}

/// Packs the parcel the initial data names.
#[derive(Step)]
struct Pack {
    #[input]
    parcel: String,
}

#[step]
impl Pack {
    async fn run(
        self,
        #[service] labels: &dyn Labels,
        #[contributor] mut contributor: Contributor<'_>,
    ) -> Result<Outcome, Error> {
        contributor.contribute("packed", labels.label(&self.parcel));
        Self::succeed()
    }
}

/// Dispatches the parcel, which may be retried while the courier cannot reach it.
#[derive(Step)]
struct Dispatch {
    #[input]
    parcel: String,
}

#[step]
impl Dispatch {
    async fn run(
        self,
        #[service] courier: &dyn Courier,
        #[service] labels: &dyn Labels,
        #[contributor] mut contributor: Contributor<'_>,
    ) -> Result<Outcome, Error> {
        let Some(carried) = courier.carry(&self.parcel) else {
            return Self::fail("unreachable");
        };
        contributor.contribute("dispatched", carried);
        contributor.contribute("labelled", labels.label(&self.parcel));
        Self::succeed()
    }
}

#[workflow(name = "courier")]
#[with_step(Pack, name = "pack")]
#[with_step(Dispatch, name = "dispatch", retry_budget = 1)]
pub(super) struct Couriers;

#[workflow]
impl Couriers {
    /// The journey's labels, made once and lent to every step and attempt.
    #[provider]
    fn labels(&self, #[journey_id] journey_id: &JourneyId) -> Result<Provided<dyn Labels>, Error> {
        Ok(provide!(Printer {
            journey_id: journey_id.to_string(),
        }))
    }

    /// A courier for each attempt to dispatch, of the company the data bag names: a van, which
    /// reaches parcels of up to five letters, on a first attempt, and a lorry, which reaches
    /// every parcel, on later ones. No company cannot be sent.
    #[provider(per_attempt, steps = ["dispatch"])]
    async fn courier(
        &self,
        #[attempt] attempt: NonZeroU32,
        #[from_workflow] company: String,
        #[reporter] reporter: AsyncHookReporter<'_>,
    ) -> Result<Provided<dyn Courier>, Error> {
        sent(&company, attempt, reporter).await
    }

    #[id_generator]
    fn parcel_id(&self, data: &DataBag) -> Result<String, Error> {
        let parcel: String = data.required("parcel")?;
        Ok(format!("parcel-{parcel}"))
    }
}

async fn sent(
    company: &str,
    attempt: NonZeroU32,
    mut reporter: AsyncHookReporter<'_>,
) -> Result<Provided<dyn Courier>, Error> {
    if company == "none" {
        return Err(Error::msg("no courier"));
    }
    let (vehicle, reach) = match attempt.get() {
        1 => ("van", 5),
        _ => ("lorry", usize::MAX),
    };
    reporter
        .info(format!("sending the {company} {vehicle}"))
        .await?;
    Ok(provide!(Vehicle {
        name: format!("{company} {vehicle}"),
        reach,
    }))
}

/// An instance of the courier workflow.
pub(super) fn instance(
    data: impl InitialData + 'static,
) -> Result<Instance<Couriers, Asynchronous>, InstanceError> {
    Couriers::builder().initial_data(data).create()
}
