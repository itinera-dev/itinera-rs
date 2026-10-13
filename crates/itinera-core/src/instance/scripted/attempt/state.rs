//! The states an attempt's script goes through as it is written, which decide what may
//! follow.

/// The state of an [`AttemptScripting`] whose step was just named: the phrases that follow
/// describe attempt 1, unless [`on_attempt`] or [`on_every_attempt`] says otherwise.
///
/// [`AttemptScripting`]: super::AttemptScripting
/// [`on_attempt`]: super::AttemptScripting::on_attempt
/// [`on_every_attempt`]: super::AttemptScripting::on_every_attempt
///
/// # Examples
///
/// ```
/// use itinera::instance::{AttemptScripting, Named, WorkflowInstance};
/// use itinera::step::{Outcome, StepDescriptor, step_name};
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Orders;
///
/// let orders = WorkflowDescriptor::builder("orders")
///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
///     .build()?;
/// let named: AttemptScripting<_, _, Named> = orders
///     .instance(Orders)
///     .create()?
///     .scripted()
///     .step("charge");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
pub enum Named {}

/// The state of an [`AttemptScripting`] whose attempts were just chosen: nothing is said yet of
/// what they do.
///
/// [`AttemptScripting`]: super::AttemptScripting
///
/// # Examples
///
/// ```
/// use itinera::instance::{AttemptScripting, Opened, WorkflowInstance};
/// use itinera::step::{Outcome, StepDescriptor, step_name};
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Orders;
///
/// let orders = WorkflowDescriptor::builder("orders")
///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
///     .build()?;
/// let opened: AttemptScripting<_, _, Opened> =
///     orders.instance(Orders).create()?.scripted().step("charge").on_every_attempt();
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
pub enum Opened {}

/// The state of an [`AttemptScripting`] that already does something, so it can no longer run
/// for real.
///
/// [`AttemptScripting`]: super::AttemptScripting
///
/// # Examples
///
/// ```
/// use itinera::instance::{Acting, AttemptScripting, WorkflowInstance};
/// use itinera::step::{Outcome, StepDescriptor, step_name};
/// use itinera::workflow::WorkflowDescriptor;
///
/// # struct Charge<'a> {
/// #     contributor: itinera::journey::Contributor<'a>,
/// #     reporter: itinera::step::StepReporter<'a>,
/// # }
/// # impl itinera::step::Step for Charge<'_> {
/// #     fn run(mut self) -> Result<Outcome, itinera::error::Error> {
/// #         self.reporter.info("charging")?;
/// #         self.contributor.contribute("charged", 40_i64);
/// #         Ok(Outcome::success())
/// #     }
/// # }
/// # struct ChargeFactory;
/// # impl itinera::step::StepFactory for ChargeFactory {
/// #     type Step<'a> = Charge<'a>;
/// #     fn needs(&self) -> itinera::step::StepNeeds {
/// #         itinera::step::StepNeeds::new().contributor().reporter()
/// #     }
/// #     fn build<'a>(
/// #         &'a self,
/// #         got: &mut itinera::step::Resolved<'a>,
/// #     ) -> Result<Charge<'a>, itinera::error::Error> {
/// #         let contributor = got.contributor()?;
/// #         let reporter = got.reporter()?;
/// #         Ok(Charge { contributor, reporter })
/// #     }
/// # }
/// // ChargeFactory builds a charge step that declares a contributor and a reporter.
///
/// struct Orders;
///
/// let orders = WorkflowDescriptor::builder("orders")
///     .step(StepDescriptor::new(step_name!("charge"), ChargeFactory))
///     .build()?;
/// let acting: AttemptScripting<_, _, Acting> = orders
///     .instance(Orders)
///     .create()?
///     .scripted()
///     .step("charge")
///     .will_contribute("charged", 40_i64);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
pub enum Acting {}
