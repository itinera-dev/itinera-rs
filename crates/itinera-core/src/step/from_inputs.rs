//! Step types built from their inputs alone.

use super::{Resolved, StepNeeds};
use crate::error::Error;

/// A step type built from its inputs alone, for each attempt: it says which inputs it needs, and
/// builds itself from them once they are resolved.
///
/// `#[derive(Step)]` implements it from the step's fields, and `#[step]` from a `new` function
/// whose parameters are inputs. The handles a step uses while it runs are not inputs: the step
/// receives them in its `run`, as [`DescribedStep`](super::DescribedStep) says.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::step::{FromInputs, Input, OptionalInput, Resolved, StepNeeds};
///
/// struct Charge {
///     amount: i64,
///     discount: Option<i64>,
/// }
///
/// const AMOUNT: Input<i64> = Input::new("amount");
/// const DISCOUNT: OptionalInput<i64> = OptionalInput::new("discount");
///
/// impl FromInputs for Charge {
///     fn needs() -> StepNeeds {
///         StepNeeds::new().input(&AMOUNT).optional_input(&DISCOUNT)
///     }
///
///     fn from_inputs<M>(got: &mut Resolved<'_, M>) -> Result<Self, Error> {
///         Ok(Charge {
///             amount: got.input(&AMOUNT)?,
///             discount: got.optional_input(&DISCOUNT)?,
///         })
///     }
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "the step `{Self}` has no way to be built from its inputs",
    label = "this step cannot be built",
    note = "derive `Step` on it, or give its `#[step]` block a `new` function whose parameters are \
            its inputs"
)]
pub trait FromInputs: Sized {
    /// The inputs the step needs, in the order it declares them.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{FromInputs, StepNeeds};
    ///
    /// fn needs<S: FromInputs>() -> StepNeeds {
    ///     S::needs()
    /// }
    /// ```
    fn needs() -> StepNeeds;

    /// Builds the step for one attempt, from its inputs once they are resolved.
    ///
    /// # Errors
    ///
    /// An error means the step could not be built, which aborts the journey with
    /// `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::step::{FromInputs, Resolved};
    ///
    /// fn build<S: FromInputs>(got: &mut Resolved<'_>) -> Result<S, Error> {
    ///     S::from_inputs(got)
    /// }
    /// ```
    fn from_inputs<M>(got: &mut Resolved<'_, M>) -> Result<Self, Error>;
}
