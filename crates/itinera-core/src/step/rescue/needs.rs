//! What a rescue step declares it needs.

use crate::step::{
    Input, InputNeed, OptionalInput, Requirement, RescueRequest, StepNeeds, StepRequest,
};
use crate::value::Value;

/// Everything a rescue step declares it needs, which the executor resolves before it is built:
/// its inputs from the data bag, what the failed attempt of the step it rescues left (the data it
/// contributed, the failure's reason and the error), and whether it needs a contributor and a
/// step reporter.
///
/// Its requests are resolved in the order they are declared, and the first required one without
/// a value aborts the journey before the rescue step is built. Declaring the reason or the error
/// again replaces the earlier declaration, and moves it to its new place in that order.
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
///     .optional_from_rescued_step(&ACCOUNT)
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

    /// Declares a required input from the data bag, after what is already declared.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Input, RescueNeeds};
    ///
    /// let needs = RescueNeeds::new().input(&Input::<String>::new("client"));
    /// ```
    pub fn input<T: Value>(self, input: &Input<T>) -> Self {
        Self {
            needs: self.needs.input(input),
        }
    }

    /// Declares an optional input from the data bag, after what is already declared.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{OptionalInput, RescueNeeds};
    ///
    /// let needs = RescueNeeds::new().optional_input(&OptionalInput::<String>::new("region"));
    /// ```
    pub fn optional_input<T: Value>(self, input: &OptionalInput<T>) -> Self {
        Self {
            needs: self.needs.optional_input(input),
        }
    }

    /// Needs data from the step it rescues: the value of type `T` its failed attempt contributed
    /// under the input's key, which was never committed. Without it, the journey is aborted with
    /// `required data missing` before the rescue step is built.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Input, RescueNeeds};
    ///
    /// let needs = RescueNeeds::new().from_rescued_step(&Input::<String>::new("account"));
    /// ```
    pub fn from_rescued_step<T: Value>(self, input: &Input<T>) -> Self {
        let need = InputNeed::required::<T>(input.key);
        Self {
            needs: self
                .needs
                .request(StepRequest::Rescued(RescueRequest::FromRescuedStep(need))),
        }
    }

    /// May use data from the step it rescues: the value of type `T` its failed attempt
    /// contributed under the input's key, if it did. Without it, the rescue step receives `None`,
    /// and `optional_input_absent` is emitted.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{OptionalInput, RescueNeeds};
    ///
    /// let needs =
    ///     RescueNeeds::new().optional_from_rescued_step(&OptionalInput::<String>::new("account"));
    /// ```
    pub fn optional_from_rescued_step<T: Value>(self, input: &OptionalInput<T>) -> Self {
        let need = InputNeed::optional::<T>(input.key);
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

    /// The needs, as the rescue step's descriptor holds them.
    pub(crate) fn into_step_needs(self) -> StepNeeds {
        self.needs
    }
}
