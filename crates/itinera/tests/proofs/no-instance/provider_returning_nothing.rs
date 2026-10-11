use itinera::error::Error;
use itinera::policy::{Provider, Requested};
use itinera::service::{PerJourney, Service, ServiceProviderDescriptor};

trait PaymentGateway: Send + Sync {}

const PAYMENTS: Service<dyn PaymentGateway> = Service::new("payments");

struct Orders;

impl Orders {
    fn payments(&self, _got: Requested<'_, Self, Provider<PerJourney>>) -> Result<(), Error> {
        Ok(())
    }
}

fn main() {
    ServiceProviderDescriptor::new("payments", &PAYMENTS, Orders::payments);
}
