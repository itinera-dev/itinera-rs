use itinera::error::Error;
use itinera::policy::{Provider, Requested};
use itinera::service::{PerAttempt, Service, ServiceProviderDescriptor};

trait PaymentGateway: Send + Sync {}

struct Gateway;

impl PaymentGateway for Gateway {}

const PAYMENTS: Service<dyn PaymentGateway> = Service::new("payments");

struct Orders;

impl Orders {
    fn payments(
        &self,
        got: Requested<'_, Self, Provider<PerAttempt>>,
    ) -> Result<Box<dyn PaymentGateway>, Error> {
        let _attempt = got.attempt();
        Ok(Box::new(Gateway))
    }
}

fn main() {
    ServiceProviderDescriptor::per_attempt("payments", &PAYMENTS, Orders::payments);
}
