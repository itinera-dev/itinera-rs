//! The delivery workflow: dispatches a parcel, asynchronously.

use itinera::error::Error;
use itinera::instance::{Instance, InstanceError};
use itinera::journey::{Contributor, InitialData};
use itinera::mode::Asynchronous;
use itinera::policy::{OnSuccess, step_policy, workflow_policy};
use itinera::step::{Outcome, Step, step};
use itinera::workflow::workflow;

/// Dispatches the parcel the initial data names.
#[derive(Step)]
struct Dispatch {
    #[input]
    parcel: String,
}

#[step]
impl Dispatch {
    async fn run(self, #[contributor] contributor: &mut Contributor) -> Result<Outcome, Error> {
        if self.parcel.is_empty() {
            return Self::fail("no-parcel");
        }
        contributor.contribute("dispatched", self.parcel);
        Self::succeed()
    }
}

/// Gives a dispatched parcel its tracking code.
#[derive(Default)]
struct Track;

#[step_policy(name = "track")]
impl Track {
    #[on_step_success]
    async fn tracked(
        &self,
        #[from_step] dispatched: String,
        #[contributor] contributor: &mut Contributor,
    ) -> Result<OnSuccess, Error> {
        contributor.contribute("tracking", format!("track {dispatched}"));
        Self::keep_default()
    }
}

/// Signs for a delivered parcel.
#[derive(Default)]
struct Sign;

#[workflow_policy(name = "sign")]
impl Sign {
    #[on_workflow_success]
    async fn signed(&self, #[contributor] contributor: &mut Contributor) -> Result<(), Error> {
        contributor.contribute("signed", true);
        Ok(())
    }
}

#[workflow(name = "delivery", policies = [Sign])]
#[with_step(Dispatch, name = "dispatch", policies = [Track])]
pub(super) struct Delivery;

/// An instance of the delivery workflow.
pub(super) fn instance(
    data: impl InitialData + 'static,
) -> Result<Instance<Delivery, Asynchronous>, InstanceError> {
    Delivery::builder().initial_data(data).create()
}
