//! What a step declares it needs, which the executor resolves before the step is built.

use super::{Input, OptionalInput};
#[cfg(feature = "unstable")]
use crate::service::Service;
use crate::value::Value;

mod input_need;
mod request;
#[cfg(feature = "unstable")]
mod service_need;

pub(crate) use input_need::{InputNeed, Requirement};
pub(crate) use request::{RescueRequest, StepRequest};
use request::{other_than_error, other_than_reason};
#[cfg(feature = "unstable")]
pub(crate) use service_need::ServiceNeed;

/// Everything a step declares it needs, which the executor resolves before the step is built:
/// its inputs, whether it needs a contributor and a step reporter, and, with the `unstable`
/// feature, its services.
///
/// Its inputs are resolved in the order they are declared, then its services, in the order they
/// are declared.
///
/// # Examples
///
/// ```
/// use itinera::step::{Input, OptionalInput, StepNeeds};
///
/// const AMOUNT: Input<i64> = Input::new("amount");
/// const DISCOUNT: OptionalInput<i64> = OptionalInput::new("discount");
///
/// let needs = StepNeeds::new()
///     .input(&AMOUNT)
///     .optional_input(&DISCOUNT)
///     .contributor()
///     .reporter();
/// ```
#[derive(Clone, Debug, Default)]
pub struct StepNeeds {
    requests: Vec<StepRequest>,
    #[cfg(feature = "unstable")]
    services: Vec<ServiceNeed>,
    contributor: bool,
    reporter: bool,
}

impl StepNeeds {
    /// Needs nothing.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::StepNeeds;
    ///
    /// let nothing = StepNeeds::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares a required input, after those already declared.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Input, StepNeeds};
    ///
    /// let needs = StepNeeds::new().input(&Input::<i64>::new("amount"));
    /// ```
    pub fn input<T: Value>(self, input: &Input<T>) -> Self {
        self.request(StepRequest::Input(InputNeed::required::<T>(input.key)))
    }

    /// Declares an optional input, after those already declared.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{OptionalInput, StepNeeds};
    ///
    /// let needs = StepNeeds::new().optional_input(&OptionalInput::<i64>::new("discount"));
    /// ```
    pub fn optional_input<T: Value>(self, input: &OptionalInput<T>) -> Self {
        self.request(StepRequest::Input(InputNeed::optional::<T>(input.key)))
    }

    /// Declares that the step needs a contributor, for the data it contributes.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::StepNeeds;
    ///
    /// let needs = StepNeeds::new().contributor();
    /// ```
    pub fn contributor(mut self) -> Self {
        self.contributor = true;
        self
    }

    /// Declares that the step needs a step reporter, for its own events.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::StepNeeds;
    ///
    /// let needs = StepNeeds::new().reporter();
    /// ```
    pub fn reporter(mut self) -> Self {
        self.reporter = true;
        self
    }

    /// Declares a service the step needs, after those already declared. The workflow decides
    /// which provider supplies it and how long an instance lives.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::service::Service;
    /// use itinera::step::StepNeeds;
    ///
    /// trait PaymentGateway: Send + Sync {}
    ///
    /// const PAYMENTS: Service<dyn PaymentGateway> = Service::new("payments");
    ///
    /// let needs = StepNeeds::new().service(&PAYMENTS);
    /// ```
    #[cfg(feature = "unstable")]
    pub fn service<T: ?Sized + Send + Sync + 'static>(mut self, service: &Service<T>) -> Self {
        self.services.push(ServiceNeed::of(service));
        self
    }

    /// What the step requests, in the order it declared it.
    pub(crate) fn requests(&self) -> &[StepRequest] {
        &self.requests
    }

    /// Requests something after what was already requested.
    pub(crate) fn request(mut self, request: StepRequest) -> Self {
        self.requests.push(request);
        self
    }

    /// Requests the failure's reason, replacing an earlier request for it.
    pub(crate) fn request_reason(mut self, requirement: Requirement) -> Self {
        self.requests.retain(other_than_reason);
        self.request(StepRequest::Rescued(RescueRequest::Reason(requirement)))
    }

    /// Requests the error, replacing an earlier request for it.
    pub(crate) fn request_error(mut self, requirement: Requirement) -> Self {
        self.requests.retain(other_than_error);
        self.request(StepRequest::Rescued(RescueRequest::Error(requirement)))
    }

    #[cfg(feature = "unstable")]
    pub(crate) fn services(&self) -> &[ServiceNeed] {
        &self.services
    }

    pub(crate) fn wants_contributor(&self) -> bool {
        self.contributor
    }

    pub(crate) fn wants_reporter(&self) -> bool {
        self.reporter
    }
}
