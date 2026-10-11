//! A scripted step as a rescue step: what it requests from the step it rescues, and its
//! attempts, which end with a rescue step's outcome.

use futures::executor::block_on;
use itinera::error::Error;
use itinera::mode::Asynchronous;
use itinera::step::{
    AsyncRescueFactory, AsyncRescueStep, AsyncStepReporter, Input, OptionalInput, RescueFactory,
    RescueNeeds, RescueOutcome, RescueResolved, RescueStep, StepReporter,
};
use itinera::value::Value as Storable;
use serde_json::Value;

use super::Scripted;
use super::attempt::Attempting;
use super::ending::End;
use super::input::requested;
use crate::model::{self, RescueRequest};
use crate::value::{Data, ForType, for_type, json};
use crate::witness::Rescued;

/// What a rescue step requests from the step it rescues, with its data typed.
#[derive(Debug)]
pub(super) enum RescueData {
    Data(Data),
    Reason,
    Error,
}

pub(super) fn requested_from_rescued(request: &RescueRequest) -> RescueData {
    match request {
        RescueRequest::Data(input) => RescueData::Data(requested(input)),
        RescueRequest::Reason => RescueData::Reason,
        RescueRequest::Error => RescueData::Error,
    }
}

/// Declares the request, with the Rust type of its data's type.
pub(super) fn needing_rescued(needs: RescueNeeds, request: &RescueData) -> RescueNeeds {
    match request {
        RescueData::Data(data) => for_type(data.value_type, Needing { needs, data }),
        RescueData::Reason => needs.reason(),
        RescueData::Error => needs.error(),
    }
}

struct Needing<'d> {
    needs: RescueNeeds,
    data: &'d Data,
}

impl ForType for Needing<'_> {
    type Output = RescueNeeds;

    fn of<T: Storable>(self) -> RescueNeeds {
        if self.data.optional {
            self.needs
                .optional_from_rescued_step(&OptionalInput::<T>::new(self.data.key))
        } else {
            self.needs
                .from_rescued_step(&Input::<T>::new(self.data.key))
        }
    }
}

/// Takes what the rescue step received for each of its requests to the step it rescues.
fn received<M>(got: &mut RescueResolved<'_, M>, requests: &[RescueData]) -> Result<Rescued, Error> {
    requests
        .iter()
        .try_fold(Rescued::default(), |rescued, request| {
            receive(got, rescued, request)
        })
}

/// Adds what the rescue step received for the request to what it received before.
fn receive<M>(
    got: &mut RescueResolved<'_, M>,
    mut rescued: Rescued,
    request: &RescueData,
) -> Result<Rescued, Error> {
    match request {
        RescueData::Data(data) => {
            let value = for_type(data.value_type, Taking { got, data })?;
            rescued.data.insert(data.key.to_owned(), value);
        }
        RescueData::Reason => rescued.reason = Some(model::Reason::written(&got.reason()?)?),
        RescueData::Error => rescued.error = Some(got.error()?.to_string()),
    }
    Ok(rescued)
}

/// Takes data from the step it rescues, as JSON, or `None` when it was absent.
struct Taking<'g, 'a, 'd, M> {
    got: &'g mut RescueResolved<'a, M>,
    data: &'d Data,
}

impl<M> ForType for Taking<'_, '_, '_, M> {
    type Output = Result<Option<Value>, Error>;

    fn of<T: Storable>(self) -> Self::Output {
        if self.data.optional {
            self.got
                .optional_from_rescued_step(&OptionalInput::<T>::new(self.data.key))?
                .map(json)
                .transpose()
        } else {
            json(
                self.got
                    .from_rescued_step(&Input::<T>::new(self.data.key))?,
            )
            .map(Some)
        }
    }
}

/// One attempt of a scripted rescue step.
#[derive(Debug, derive_more::From)]
pub(crate) struct RescueAttempt<'a, R> {
    attempting: Attempting<'a, R>,
}

impl RescueStep for RescueAttempt<'_, StepReporter<'_>> {
    fn run(self) -> Result<RescueOutcome, Error> {
        block_on(self.attempting.perform()).and_then(End::rescue_outcome)
    }
}

impl AsyncRescueStep for RescueAttempt<'_, AsyncStepReporter<'_>> {
    async fn run(self) -> Result<RescueOutcome, Error> {
        self.attempting
            .perform()
            .await
            .and_then(End::rescue_outcome)
    }
}

impl RescueFactory for Scripted {
    type Step<'a> = RescueAttempt<'a, StepReporter<'a>>;

    fn needs(&self) -> RescueNeeds {
        self.script.rescue_needs()
    }

    fn build<'a>(&'a self, got: &mut RescueResolved<'a>) -> Result<Self::Step<'a>, Error> {
        let rescued = received(got, &self.script.rescue_requests)?;
        self.attempting(got, rescued).map(RescueAttempt::from)
    }
}

impl AsyncRescueFactory for Scripted {
    type Step<'a> = RescueAttempt<'a, AsyncStepReporter<'a>>;

    fn needs(&self) -> RescueNeeds {
        self.script.rescue_needs()
    }

    fn build<'a>(
        &'a self,
        got: &mut RescueResolved<'a, Asynchronous>,
    ) -> Result<Self::Step<'a>, Error> {
        let rescued = received(got, &self.script.rescue_requests)?;
        self.attempting(got, rescued).map(RescueAttempt::from)
    }
}
