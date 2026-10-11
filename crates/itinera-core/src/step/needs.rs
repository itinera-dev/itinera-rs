//! What a step declares it needs, which the executor resolves before the step is built.

use super::{Input, OptionalInput};
use crate::value::{AnyValue, Value};

mod request;

pub(crate) use request::{RescueRequest, StepRequest};
use request::{other_than_error, other_than_reason};

/// Everything a step declares it needs, which the executor resolves before the step is built:
/// its inputs, and whether it needs a contributor and a step reporter.
///
/// Its inputs are resolved in the order they are declared.
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
    contributor: bool,
    reporter: bool,
}

/// Whether a step needs an input to be built, or may be built without it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Requirement {
    Required,
    Optional,
}

/// One declared input, as the executor resolves it.
#[derive(Clone, derive_more::Debug)]
pub(crate) struct InputNeed {
    key: &'static str,
    requirement: Requirement,
    #[debug(skip)]
    accepts: fn(&AnyValue) -> bool,
}

impl InputNeed {
    /// A value of type `T` under the key, without which nothing can be built.
    pub(crate) fn required<T: Value>(key: &'static str) -> Self {
        Self {
            key,
            requirement: Requirement::Required,
            accepts: AnyValue::is::<T>,
        }
    }

    /// A value of type `T` under the key, which may be absent.
    pub(crate) fn optional<T: Value>(key: &'static str) -> Self {
        Self {
            key,
            requirement: Requirement::Optional,
            accepts: AnyValue::is::<T>,
        }
    }

    pub(crate) fn key(&self) -> &'static str {
        self.key
    }

    pub(crate) fn requirement(&self) -> Requirement {
        self.requirement
    }

    /// Whether a value has the type the input declares.
    pub(crate) fn accepts(&self, value: &AnyValue) -> bool {
        (self.accepts)(value)
    }
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

    pub(crate) fn wants_contributor(&self) -> bool {
        self.contributor
    }

    pub(crate) fn wants_reporter(&self) -> bool {
        self.reporter
    }
}
