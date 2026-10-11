use itinera::error::Error;
use itinera::policy::{HookNeeds, Provider, Requested};
use itinera::service::{PerJourney, Service, ServiceProviderDescriptor};

trait PaymentGateway: Send + Sync {}

struct Gateway;

impl PaymentGateway for Gateway {}

const PAYMENTS: Service<dyn PaymentGateway> = Service::new("payments");

struct Orders;

impl Orders {
    fn payments(
        &self,
        mut got: Requested<'_, Self, Provider<PerJourney>>,
    ) -> Result<Box<dyn PaymentGateway>, Error> {
        got.reporter()?.info_with("odd data", 1_i64)?;
        Ok(Box::new(Gateway))
    }
}

fn main() {
    ServiceProviderDescriptor::new("payments", &PAYMENTS, Orders::payments)
        .needing(HookNeeds::new().reporter());
}
