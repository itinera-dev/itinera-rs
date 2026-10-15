//! What a rescue step declares it needs.

use crate::service::Service;
use crate::step::{InputNeed, InputToken, Requirement, RescueRequest, StepNeeds, StepRequest};

/// Everything a rescue step declares it needs, which the executor resolves before it is built:
/// its inputs, from the input adapter attached to its name or else the data bag, what the failed
/// attempt of the step it rescues left (the data it contributed, the failure's reason and the
/// error), whether it needs a contributor and a step reporter, and its services.
///
/// Its requests are resolved in the order they are declared, then its services, in the order they
/// are declared, and the first required request without a value aborts the journey before the
/// rescue step is built. Declaring the reason or the error again replaces the earlier declaration,
/// and moves it to its new place in that order.
///
/// # Examples
///
/// ```
/// use itinera::step::{Input, OptionalInput, RescueNeeds};
///
/// const CLIENT: Input<String> = Input::new("client");
/// const ACCOUNT: OptionalInput<String> = OptionalInput::new("account");
///
/// let needs = RescueNeeds::new()
///     .input(&CLIENT)
///     .from_rescued_step(&ACCOUNT)
///     .reason()
///     .contributor();
/// ```
#[derive(Clone, Debug, Default)]
pub struct RescueNeeds {
    needs: StepNeeds,
}

impl RescueNeeds {
    /// Needs nothing.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::RescueNeeds;
    ///
    /// let nothing = RescueNeeds::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares an input, after what is already declared: required for an [`Input`], optional
    /// for an [`OptionalInput`]. The input adapter attached to the rescue step's name supplies
    /// it, if there is one and it returns a value; otherwise it is read from the data bag. An
    /// adapter attached to the step it rescues is never asked.
    ///
    /// [`Input`]: crate::step::Input
    /// [`OptionalInput`]: crate::step::OptionalInput
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Input, OptionalInput, RescueNeeds};
    ///
    /// let needs = RescueNeeds::new()
    ///     .input(&Input::<String>::new("client"))
    ///     .input(&OptionalInput::<String>::new("region"));
    /// ```
    pub fn input(self, input: &impl InputToken) -> Self {
        Self {
            needs: self.needs.input(input),
        }
    }

    /// Needs data from the step it rescues: the value of the input's type its failed attempt
    /// contributed under the input's key, which was never committed. Without it, the journey is
    /// aborted with `required data missing` before the rescue step is built, for an [`Input`]; for
    /// an [`OptionalInput`], the rescue step receives `None`, and `optional_input_absent` is
    /// emitted.
    ///
    /// [`Input`]: crate::step::Input
    /// [`OptionalInput`]: crate::step::OptionalInput
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Input, OptionalInput, RescueNeeds};
    ///
    /// let needs = RescueNeeds::new()
    ///     .from_rescued_step(&Input::<String>::new("account"))
    ///     .from_rescued_step(&OptionalInput::<String>::new("note"));
    /// ```
    pub fn from_rescued_step(self, input: &impl InputToken) -> Self {
        let need = InputNeed::of(input);
        Self {
            needs: self
                .needs
                .request(StepRequest::Rescued(RescueRequest::FromRescuedStep(need))),
        }
    }

    /// Needs the reason of the failure it rescues. When the attempt reported no failure, because
    /// it ended in an abnormal termination, the journey is aborted with `required data missing`
    /// before the rescue step is built.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::RescueNeeds;
    ///
    /// let needs = RescueNeeds::new().reason();
    /// ```
    pub fn reason(self) -> Self {
        Self {
            needs: self.needs.request_reason(Requirement::Required),
        }
    }

    /// May use the reason of the failure it rescues, if the attempt reported one.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::RescueNeeds;
    ///
    /// let needs = RescueNeeds::new().optional_reason();
    /// ```
    pub fn optional_reason(self) -> Self {
        Self {
            needs: self.needs.request_reason(Requirement::Optional),
        }
    }

    /// Needs the error that ended the attempt it rescues abnormally. When the attempt did not end
    /// in an abnormal termination, the journey is aborted with `required data missing` before the
    /// rescue step is built.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::RescueNeeds;
    ///
    /// let needs = RescueNeeds::new().error();
    /// ```
    pub fn error(self) -> Self {
        Self {
            needs: self.needs.request_error(Requirement::Required),
        }
    }

    /// May use the error that ended the attempt it rescues abnormally, if it did.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::RescueNeeds;
    ///
    /// let needs = RescueNeeds::new().optional_error();
    /// ```
    pub fn optional_error(self) -> Self {
        Self {
            needs: self.needs.request_error(Requirement::Optional),
        }
    }

    /// Declares that the rescue step needs a contributor, for the data it contributes.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::RescueNeeds;
    ///
    /// let needs = RescueNeeds::new().contributor();
    /// ```
    pub fn contributor(self) -> Self {
        Self {
            needs: self.needs.contributor(),
        }
    }

    /// Declares that the rescue step needs a step reporter, for its own events.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::RescueNeeds;
    ///
    /// let needs = RescueNeeds::new().reporter();
    /// ```
    pub fn reporter(self) -> Self {
        Self {
            needs: self.needs.reporter(),
        }
    }

    /// Declares a service the rescue step needs, after those already declared. The workflow's
    /// provider of its key attached to the rescue step's name serves it, or else the one attached
    /// to no step; a provider attached to the step it rescues never does.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::service::Service;
    /// use itinera::step::RescueNeeds;
    ///
    /// trait PaymentGateway: Send + Sync {}
    ///
    /// const PAYMENTS: Service<dyn PaymentGateway> = Service::new("payments");
    ///
    /// let needs = RescueNeeds::new().service(&PAYMENTS);
    /// ```
    pub fn service<T: ?Sized + Send + Sync + 'static>(self, service: &Service<T>) -> Self {
        Self {
            needs: self.needs.service(service),
        }
    }

    /// The needs, as the rescue step's descriptor holds them.
    pub(crate) fn into_step_needs(self) -> StepNeeds {
        self.needs
    }
}
