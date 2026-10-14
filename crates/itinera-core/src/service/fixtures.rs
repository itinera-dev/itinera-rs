//! Fixtures the tests of several modules share: a service that tells its label, one that counts
//! its instances dropped, the providers that make them, steps and rescue steps that request
//! them, and the implementations `service_provided` names.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::{
    Implementation, PerAttempt, PerJourney, Provided, Service, ServiceProviderDescriptor, provide,
};
use crate::error::Error;
use crate::event::{Event, EventBody};
use crate::step::{
    Outcome, RescueFactory, RescueNeeds, RescueOutcome, RescueResolved, RescueStep, Resolved, Step,
    StepDescriptor, StepFactory, StepName, StepNeeds,
};
use crate::value::AnyValue;
use crate::workflow::fixtures::Orders;

/// A service with one operation, which tells which instance it is.
pub(crate) trait Labeller: Send + Sync {
    fn label(&self) -> String;
}

/// The instance [`labelled`] makes.
pub(crate) struct Label {
    label: String,
}

impl Labeller for Label {
    fn label(&self) -> String {
        self.label.clone()
    }
}

pub(crate) const PAYMENTS: Service<dyn Labeller> = Service::new("payments");
pub(crate) const STOCK: Service<dyn Labeller> = Service::new("stock");
/// The payments service as a number, another type than `PAYMENTS`.
pub(crate) const PAYMENTS_AS_NUMBER: Service<i64> = Service::new("payments");

/// How many instances a provider made.
pub(crate) type Made = Arc<AtomicUsize>;

pub(crate) fn made(made: &Made) -> usize {
    made.load(Ordering::SeqCst)
}

/// A provider of journey scope, attached to no step, labelling each instance with its name and
/// how many it made, such as `stock 1`.
pub(crate) fn per_journey(
    name: &'static str,
    service: &Service<dyn Labeller>,
    made: &Made,
) -> ServiceProviderDescriptor<Orders, PerJourney> {
    let made = Arc::clone(made);
    ServiceProviderDescriptor::new(name, service, move |_, _| Ok(labelled(name, &made)))
}

/// A provider of attempt scope, attached to no step, labelling each instance as
/// [`per_journey`] does.
pub(crate) fn per_attempt(
    name: &'static str,
    service: &Service<dyn Labeller>,
    made: &Made,
) -> ServiceProviderDescriptor<Orders, PerAttempt> {
    let made = Arc::clone(made);
    ServiceProviderDescriptor::per_attempt(name, service, move |_, _| Ok(labelled(name, &made)))
}

pub(crate) fn labelled(name: &str, made: &AtomicUsize) -> Provided<dyn Labeller> {
    let count = made.fetch_add(1, Ordering::SeqCst) + 1;
    provide!(Label {
        label: format!("{name} {count}"),
    })
}

/// How many instances of a service were dropped.
pub(crate) type Dropped = Arc<AtomicUsize>;

pub(crate) fn drops(dropped: &Dropped) -> usize {
    dropped.load(Ordering::SeqCst)
}

/// An instance that counts itself in `dropped` once it is dropped.
pub(crate) fn dropping(dropped: &Dropped) -> Provided<dyn Labeller> {
    provide!(Dropping {
        dropped: Arc::clone(dropped),
    })
}

struct Dropping {
    dropped: Dropped,
}

impl Labeller for Dropping {
    fn label(&self) -> String {
        "dropping".to_owned()
    }
}

impl Drop for Dropping {
    fn drop(&mut self) {
        self.dropped.fetch_add(1, Ordering::SeqCst);
    }
}

/// The labels of the services each attempt of a step was given, in the order it took them.
pub(crate) type Given = Arc<Mutex<Vec<String>>>;

pub(crate) fn given(given: &Given) -> Vec<String> {
    given.lock().unwrap().clone()
}

/// How an attempt of a step ends.
pub(crate) type Ends = fn() -> Result<Outcome, Error>;

/// A step that requests `services`, records the label of each in `given` when it is built, and
/// ends each attempt as the next of `ends` says, succeeding once none is left.
pub(crate) fn requesting(
    step: &'static str,
    services: &[&'static Service<dyn Labeller>],
    ends: &'static [Ends],
    given: &Given,
) -> StepDescriptor<Orders> {
    StepDescriptor::new(
        StepName::new(step),
        Requesting {
            services: services.to_vec(),
            ends,
            attempts: AtomicUsize::new(0),
            given: Arc::clone(given),
        },
    )
}

struct Requesting {
    services: Vec<&'static Service<dyn Labeller>>,
    ends: &'static [Ends],
    attempts: AtomicUsize,
    given: Given,
}

struct Ending {
    end: Option<Ends>,
}

impl StepFactory for Requesting {
    type Step<'a> = Ending;

    fn needs(&self) -> StepNeeds {
        self.services
            .iter()
            .copied()
            .fold(StepNeeds::new(), StepNeeds::service)
    }

    fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Ending, Error> {
        let labels = self
            .services
            .iter()
            .map(|service| label_of(got, service))
            .collect::<Result<Vec<_>, _>>()?;
        self.given.lock().unwrap().extend(labels);
        let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
        Ok(Ending {
            end: self.ends.get(attempt).copied(),
        })
    }
}

fn label_of(got: &Resolved<'_>, service: &Service<dyn Labeller>) -> Result<String, Error> {
    Ok(got.service(service)?.label())
}

impl Step for Ending {
    fn run(self) -> Result<Outcome, Error> {
        self.end.map_or_else(succeed, call)
    }
}

fn succeed() -> Result<Outcome, Error> {
    Ok(Outcome::success())
}

fn call(end: Ends) -> Result<Outcome, Error> {
    end()
}

/// Gives the step a rescue step that requests `services`, records the label of each in `given`
/// when it is built, and succeeds.
pub(crate) fn rescued_requesting(
    step: StepDescriptor<Orders>,
    services: &[&'static Service<dyn Labeller>],
    given: &Given,
) -> StepDescriptor<Orders> {
    step.rescue(RescueRequesting {
        services: services.to_vec(),
        given: Arc::clone(given),
    })
}

struct RescueRequesting {
    services: Vec<&'static Service<dyn Labeller>>,
    given: Given,
}

struct Rescuing;

impl RescueFactory for RescueRequesting {
    type Step<'a> = Rescuing;

    fn needs(&self) -> RescueNeeds {
        self.services
            .iter()
            .copied()
            .fold(RescueNeeds::new(), RescueNeeds::service)
    }

    fn build<'a>(&'a self, got: &mut RescueResolved<'a>) -> Result<Rescuing, Error> {
        let labels = self
            .services
            .iter()
            .map(|service| rescue_label_of(got, service))
            .collect::<Result<Vec<_>, _>>()?;
        self.given.lock().unwrap().extend(labels);
        Ok(Rescuing)
    }
}

fn rescue_label_of(
    got: &RescueResolved<'_>,
    service: &Service<dyn Labeller>,
) -> Result<String, Error> {
    Ok(got.service(service)?.label())
}

impl RescueStep for Rescuing {
    fn run(self) -> Result<RescueOutcome, Error> {
        Ok(RescueOutcome::success())
    }
}

/// The implementation of every `service_provided`, in order, with the data it carries when that
/// is a string.
pub(crate) fn implementations(events: &[Event]) -> Vec<(Implementation, Option<String>)> {
    events.iter().filter_map(implementation).collect()
}

fn implementation(event: &Event) -> Option<(Implementation, Option<String>)> {
    match &event.body {
        EventBody::ServiceProvided {
            implementation,
            data,
            ..
        } => Some((*implementation, data.as_ref().and_then(as_string).cloned())),
        _ => None,
    }
}

fn as_string(data: &AnyValue) -> Option<&String> {
    data.downcast_ref()
}
