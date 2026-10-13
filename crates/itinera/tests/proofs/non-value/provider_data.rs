use itinera::error::Error;
use itinera::policy::{Provider, Requested};
use itinera::service::{PerJourney, Provided, Service, ServiceProviderDescriptor, provide};

trait PaymentGateway: Send + Sync {}

struct Gateway;

impl PaymentGateway for Gateway {}

const PAYMENTS: Service<dyn PaymentGateway> = Service::new("payments");

struct Orders;

impl Orders {
    fn payments(
        &self,
        _got: Requested<'_, Self, Provider<PerJourney>>,
    ) -> Result<Provided<dyn PaymentGateway>, Error> {
        Ok(provide!(Gateway, || 1))
    }
}

fn main() {
    ServiceProviderDescriptor::new("payments", &PAYMENTS, Orders::payments);
}
