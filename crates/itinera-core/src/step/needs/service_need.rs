//! One service a step declares.

use crate::service::{Service, ServiceType};

/// One declared service, as admission checks it and the executor resolves it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ServiceNeed {
    key: &'static str,
    service_type: ServiceType,
}

impl ServiceNeed {
    /// The service under the token's key, of the token's type.
    pub(super) fn of<T: ?Sized + Send + Sync + 'static>(service: &Service<T>) -> Self {
        Self {
            key: service.key(),
            service_type: ServiceType::of::<T>(),
        }
    }

    pub(crate) fn key(&self) -> &'static str {
        self.key
    }

    pub(crate) fn service_type(&self) -> ServiceType {
        self.service_type
    }
}
