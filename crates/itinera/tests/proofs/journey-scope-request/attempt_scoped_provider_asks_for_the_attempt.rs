use itinera::error::Error;
use itinera::policy::Requested;
use itinera::service::{
    PerAttempt, Provided, Provider, Service, ServiceProviderDescriptor, provide,
};

trait PaymentGateway: Send + Sync {}

struct Gateway;

impl PaymentGateway for Gateway {}

const PAYMENTS: Service<dyn PaymentGateway> = Service::new("payments");

struct Orders;

impl Orders {
    fn payments(
        &self,
        got: Requested<'_, Self, Provider<PerAttempt>>,
    ) -> Result<Provided<dyn PaymentGateway>, Error> {
        let _attempt = got.attempt();
        Ok(provide!(Gateway))
    }
}

pub(crate) fn main() {
    ServiceProviderDescriptor::per_attempt("payments", &PAYMENTS, Orders::payments);
}
