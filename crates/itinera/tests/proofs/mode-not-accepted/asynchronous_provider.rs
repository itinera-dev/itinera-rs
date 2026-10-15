use itinera::error::Error;
use itinera::mode::Asynchronous;
use itinera::policy::Requested;
use itinera::service::{
    PerJourney, Provided, Provider, Service, ServiceProviderDescriptor, provide,
};
use itinera::workflow::WorkflowDescriptor;

trait PaymentGateway: Send + Sync {}

struct Gateway;

impl PaymentGateway for Gateway {}

const PAYMENTS: Service<dyn PaymentGateway> = Service::new("payments");

struct Orders;

impl Orders {
    async fn payments(
        &self,
        _got: Requested<'_, Self, Provider<PerJourney>, Asynchronous>,
    ) -> Result<Provided<dyn PaymentGateway>, Error> {
        Ok(provide!(Gateway))
    }
}

fn main() {
    let _orders = WorkflowDescriptor::<Orders>::builder("orders")
        .provider(ServiceProviderDescriptor::new_async("payments", &PAYMENTS, Orders::payments));
}
