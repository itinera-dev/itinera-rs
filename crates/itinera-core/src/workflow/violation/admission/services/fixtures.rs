//! Fixtures the tests of admission's checks on services share: steps that request services, and
//! the workflows and providers that serve them.

use crate::service::fixtures::{
    Given, Labeller, Made, PAYMENTS, PAYMENTS_AS_NUMBER, per_journey, requesting,
    rescued_requesting,
};
use crate::service::{PerJourney, Service, ServiceProviderDescriptor, provide};
use crate::step::StepDescriptor;
use crate::workflow::fixtures::{Orders, orders};
use crate::workflow::{Violation, WorkflowBuilder};

pub(super) fn step(
    name: &'static str,
    services: &[&'static Service<dyn Labeller>],
) -> StepDescriptor<Orders> {
    requesting(name, services, &[], &Given::default())
}

pub(super) fn payments(name: &'static str) -> ServiceProviderDescriptor<Orders, PerJourney> {
    per_journey(name, &PAYMENTS, &Made::default())
}

/// A provider of the payments service as a number, another type than steps request.
pub(super) fn numbered_payments() -> ServiceProviderDescriptor<Orders, PerJourney> {
    ServiceProviderDescriptor::new("numbers", &PAYMENTS_AS_NUMBER, |_: &Orders, _| {
        Ok(provide!(1))
    })
}

/// The orders workflow with one step, charge, which requests the payments service.
pub(super) fn charging() -> WorkflowBuilder<Orders> {
    orders().step(step("charge", &[&PAYMENTS]))
}

/// The orders workflow with one step, charge, which requests no service.
pub(super) fn charge_requesting_nothing() -> WorkflowBuilder<Orders> {
    orders().step(step("charge", &[]))
}

/// The orders workflow with one step, charge, which requests `services`, as its rescue step
/// does `rescue_services`.
pub(super) fn rescued_charge(
    services: &[&'static Service<dyn Labeller>],
    rescue_services: &[&'static Service<dyn Labeller>],
) -> WorkflowBuilder<Orders> {
    orders().step(rescued_requesting(
        step("charge", services),
        rescue_services,
        &Given::default(),
    ))
}

pub(super) fn violations(builder: WorkflowBuilder<Orders>) -> Vec<Violation> {
    builder.build().unwrap_err().into_iter().collect()
}
