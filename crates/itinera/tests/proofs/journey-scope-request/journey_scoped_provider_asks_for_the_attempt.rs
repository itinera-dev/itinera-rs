use itinera::error::Error;
use itinera::policy::{Provider, Requested};
use itinera::service::{PerJourney, Service, ServiceProviderDescriptor};

trait PaymentGateway: Send + Sync {}

struct Gateway;

impl PaymentGateway for Gateway {}

const PAYMENTS: Service<dyn PaymentGateway> = Service::new("payments");

struct Orders;

impl Orders {
    fn payments(
        &self,
        got: Requested<'_, Self, Provider<PerJourney>>,
    ) -> Result<Box<dyn PaymentGateway>, Error> {
        let _attempt = got.attempt();
        Ok(Box::new(Gateway))
    }
}

fn main() {
    ServiceProviderDescriptor::new("payments", &PAYMENTS, Orders::payments);
}
