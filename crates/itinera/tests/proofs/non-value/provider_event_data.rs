use itinera::error::Error;
use itinera::policy::{HookNeeds, Provider, Requested};
use itinera::service::{PerJourney, Provided, Service, ServiceProviderDescriptor, provide};

trait PaymentGateway: Send + Sync {}

struct Gateway;

impl PaymentGateway for Gateway {}

const PAYMENTS: Service<dyn PaymentGateway> = Service::new("payments");

struct Orders;

impl Orders {
    fn payments(
        &self,
        mut got: Requested<'_, Self, Provider<PerJourney>>,
    ) -> Result<Provided<dyn PaymentGateway>, Error> {
        got.reporter()?.info_with("odd data", || 1)?;
        Ok(provide!(Gateway))
    }
}

fn main() {
    ServiceProviderDescriptor::new("payments", &PAYMENTS, Orders::payments)
        .needing(HookNeeds::new().reporter());
}
