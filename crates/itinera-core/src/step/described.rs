//! Step types that know how a workflow describes them, and the shorthands for their outcomes.

use super::{Outcome, Reason, StepDescriptor, StepName};
use crate::error::Error;
use crate::mode::Mode;

/// A step type that knows how a workflow describes it: given only the name the workflow gives
/// it, it makes its step descriptor, which says what it needs and how to build and run it for
/// each attempt. A workflow can then list the step by its type.
///
/// `#[step]` implements it. Every step type implementing it also has the shorthands for its
/// outcomes, such as `Self::succeed()` and `Self::fail("declined")`, which `#[step]` brings into
/// scope in the functions of the block it is written on.
///
/// # Examples
///
/// ```
/// use itinera::error::Result;
/// use itinera::mode::Synchronous;
/// use itinera::step::{
///     DescribedStep, Outcome, Resolved, Step, StepDescriptor, StepFactory, StepName, StepNeeds,
///     step_name,
/// };
///
/// struct Ship;
///
/// impl Step for Ship {
///     fn run(self) -> Result<Outcome> {
///         Self::succeed()
///     }
/// }
///
/// struct ShipFactory;
///
/// impl StepFactory for ShipFactory {
///     type Step<'a> = Ship;
///
///     fn needs(&self) -> StepNeeds {
///         StepNeeds::new()
///     }
///
///     fn build<'a>(&'a self, _got: &mut Resolved<'a>) -> Result<Ship> {
///         Ok(Ship)
///     }
/// }
///
/// impl DescribedStep for Ship {
///     type Mode = Synchronous;
///
///     fn descriptor<W>(name: StepName) -> StepDescriptor<W> {
///         StepDescriptor::new(name, ShipFactory)
///     }
/// }
///
/// struct Orders;
///
/// let ship = Ship::descriptor::<Orders>(step_name!("ship"));
/// assert_eq!(ship.name(), step_name!("ship"));
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a step a workflow can list by its type",
    label = "not a described step",
    note = "write `#[step]` on the block that holds its `run`"
)]
pub trait DescribedStep {
    /// The mode of the workflows the step can be part of: an asynchronous step only belongs to
    /// an asynchronous workflow.
    type Mode: Mode;

    /// The keys of the services the step requests, in the order it declares them: none unless an
    /// implementation says otherwise. `#[step]` lists them, so that `#[workflow]` refuses to
    /// compile a workflow that provides no service for one of them; the descriptor's `build()`
    /// checks every step either way.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Result;
    /// use itinera::mode::Synchronous;
    /// use itinera::service::Service;
    /// use itinera::step::{
    ///     DescribedStep, Outcome, Resolved, Step, StepDescriptor, StepFactory, StepName, StepNeeds,
    /// };
    ///
    /// trait PaymentGateway: Send + Sync {}
    ///
    /// const PAYMENTS: Service<dyn PaymentGateway> = Service::new("payments");
    ///
    /// struct Charge<'a> {
    ///     payments: &'a dyn PaymentGateway,
    /// }
    ///
    /// impl Step for Charge<'_> {
    ///     fn run(self) -> Result<Outcome> {
    ///         Ok(Outcome::success())
    ///     }
    /// }
    ///
    /// struct Charging;
    ///
    /// impl StepFactory for Charging {
    ///     type Step<'a> = Charge<'a>;
    ///
    ///     fn needs(&self) -> StepNeeds {
    ///         StepNeeds::new().service(&PAYMENTS)
    ///     }
    ///
    ///     fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Charge<'a>> {
    ///         Ok(Charge {
    ///             payments: got.service(&PAYMENTS)?,
    ///         })
    ///     }
    /// }
    ///
    /// struct Charged;
    ///
    /// impl DescribedStep for Charged {
    ///     type Mode = Synchronous;
    ///
    ///     const SERVICES: &'static [&'static str] = &["payments"];
    ///
    ///     fn descriptor<W>(name: StepName) -> StepDescriptor<W> {
    ///         StepDescriptor::new(name, Charging)
    ///     }
    /// }
    ///
    /// assert_eq!(Charged::SERVICES, ["payments"]);
    /// ```
    const SERVICES: &'static [&'static str] = &[];

    /// Describes the step under the name the workflow gives it, with no retries and no policies,
    /// as [`StepDescriptor`]'s constructors do.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{DescribedStep, StepDescriptor, step_name};
    ///
    /// fn charge<S: DescribedStep, W>() -> StepDescriptor<W, S::Mode> {
    ///     S::descriptor(step_name!("charge")).retry_budget(2)
    /// }
    /// ```
    fn descriptor<W>(name: StepName) -> StepDescriptor<W, Self::Mode>;

    /// The step succeeded: [`Outcome::success`].
    ///
    /// # Errors
    ///
    /// Never; it returns a `Result` so that a step's `run` can return it as it is.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Result;
    /// use itinera::step::{DescribedStep, Outcome};
    ///
    /// fn done<S: DescribedStep>() -> Result<Outcome> {
    ///     S::succeed()
    /// }
    /// ```
    fn succeed() -> Result<Outcome, Error> {
        Ok(Outcome::success())
    }

    /// The step failed, for this reason: [`Outcome::failure`]. A text is the reason's code.
    ///
    /// # Errors
    ///
    /// Never; it returns a `Result` so that a step's `run` can return it as it is.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Result;
    /// use itinera::step::{DescribedStep, Outcome};
    ///
    /// fn declined<S: DescribedStep>() -> Result<Outcome> {
    ///     S::fail("card-declined")
    /// }
    /// ```
    fn fail(reason: impl Into<Reason>) -> Result<Outcome, Error> {
        Ok(Outcome::failure(reason))
    }

    /// The step had nothing to do: [`Outcome::skipped`].
    ///
    /// # Errors
    ///
    /// Never; it returns a `Result` so that a step's `run` can return it as it is.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Result;
    /// use itinera::step::{DescribedStep, Outcome};
    ///
    /// fn nothing_to_do<S: DescribedStep>() -> Result<Outcome> {
    ///     S::skip()
    /// }
    /// ```
    fn skip() -> Result<Outcome, Error> {
        Ok(Outcome::skipped())
    }

    /// The step had nothing to do, for this reason: [`Outcome::skipped_because`]. A text is the
    /// reason's code.
    ///
    /// # Errors
    ///
    /// Never; it returns a `Result` so that a step's `run` can return it as it is.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Result;
    /// use itinera::step::{DescribedStep, Outcome};
    ///
    /// fn already_paid<S: DescribedStep>() -> Result<Outcome> {
    ///     S::skip_because("already-paid")
    /// }
    /// ```
    fn skip_because(reason: impl Into<Reason>) -> Result<Outcome, Error> {
        Ok(Outcome::skipped_because(reason))
    }
}
