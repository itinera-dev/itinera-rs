//! Rescue step types built from their requests alone.

use super::{RescueNeeds, RescueResolved};
use crate::error::Error;

/// A rescue step type built from its requests alone: it says what it requests, its inputs and what
/// it requests from the step it rescues, and builds itself from them once they are resolved.
///
/// `#[derive(RescueStep)]` implements it from the rescue step's fields. The handles a rescue step
/// uses while it runs are not requests: the rescue step receives them in its `run`, as
/// [`DescribedRescueStep`](super::DescribedRescueStep) says.
///
/// # Examples
///
/// ```
/// use itinera::error::Result;
/// use itinera::step::{FromRescueRequests, Input, Reason, RescueNeeds, RescueResolved};
///
/// struct RefreshToken {
///     account: String,
///     reason: Option<Reason>,
/// }
///
/// const ACCOUNT: Input<String> = Input::new("account");
///
/// impl FromRescueRequests for RefreshToken {
///     fn needs() -> RescueNeeds {
///         RescueNeeds::new().from_rescued_step(&ACCOUNT).optional_reason()
///     }
///
///     fn from_requests<M>(got: &mut RescueResolved<'_, M>) -> Result<Self> {
///         Ok(RefreshToken {
///             account: got.from_rescued_step(&ACCOUNT)?,
///             reason: got.optional_reason()?,
///         })
///     }
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "the rescue step `{Self}` has no way to be built from its requests",
    label = "this rescue step cannot be built",
    note = "derive `RescueStep` on it"
)]
pub trait FromRescueRequests: Sized {
    /// What the rescue step requests, in the order it declares it.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{FromRescueRequests, RescueNeeds};
    ///
    /// fn needs<R: FromRescueRequests>() -> RescueNeeds {
    ///     R::needs()
    /// }
    /// ```
    fn needs() -> RescueNeeds;

    /// Builds the rescue step, from what it requested once it is resolved.
    ///
    /// # Errors
    ///
    /// An error means the rescue step could not be built, which aborts the journey with
    /// `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Result;
    /// use itinera::step::{FromRescueRequests, RescueResolved};
    ///
    /// fn build<R: FromRescueRequests>(got: &mut RescueResolved<'_>) -> Result<R> {
    ///     R::from_requests(got)
    /// }
    /// ```
    fn from_requests<M>(got: &mut RescueResolved<'_, M>) -> Result<Self, Error>;
}
