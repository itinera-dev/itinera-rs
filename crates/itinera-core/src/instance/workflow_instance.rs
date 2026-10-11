//! What an executor runs: a workflow instance, whoever made it.

use crate::journey::{DataBag, JourneyId};
use crate::mode::Mode;
use crate::value::AnyValue;
use crate::workflow::WorkflowDescriptor;

/// What an executor runs: the instance of a workflow for one journey, holding everything that
/// belongs to that journey.
///
/// This trait is all an executor relies on, so an executor runs any instance, however it was
/// made. [`Instance`] is the one itinera makes; a hand-written instance implements this trait
/// itself, and points at a descriptor built by [`WorkflowBuilder::build`].
///
/// An instance runs one journey: running it takes it by value.
///
/// [`Instance`]: super::Instance
/// [`WorkflowBuilder::build`]: crate::workflow::WorkflowBuilder::build
///
/// # Examples
///
/// ```
/// use itinera::instance::WorkflowInstance;
///
/// fn describe(instance: &impl WorkflowInstance) -> String {
///     format!("{} for {}", instance.descriptor().name(), instance.journey_id())
/// }
/// ```
pub trait WorkflowInstance: Send + Sized + 'static {
    /// The workflow's own type, which holds what its code needs.
    type Workflow: Send + Sync + 'static;
    /// The workflow's execution mode.
    type Mode: Mode;

    /// The workflow's descriptor.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::workflow::WorkflowName;
    ///
    /// fn workflow_name(instance: &impl WorkflowInstance) -> WorkflowName {
    ///     instance.descriptor().name()
    /// }
    /// ```
    fn descriptor(&self) -> &WorkflowDescriptor<Self::Workflow, Self::Mode>;

    /// The workflow's own value.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    ///
    /// struct Orders {
    ///     region: String,
    /// }
    ///
    /// fn region(instance: &impl WorkflowInstance<Workflow = Orders>) -> &str {
    ///     &instance.workflow().region
    /// }
    /// ```
    fn workflow(&self) -> &Self::Workflow;

    /// The journey's ID, produced when the instance was created.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .id_generator(|_: &Orders, _| Ok("order-1".to_string()))
    ///     .build()?;
    /// let instance = orders.instance(Orders).create()?;
    /// assert_eq!(instance.journey_id().to_string(), "order-1");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    fn journey_id(&self) -> &JourneyId;

    /// Hands the journey's reporters over, in the order the workflow lists them. Called once, by
    /// the executor, when the journey starts.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders").build()?;
    /// let mut instance = orders.instance(Orders).create()?;
    /// assert!(instance.take_reporters().is_empty());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    fn take_reporters(&mut self) -> Vec<<Self::Mode as Mode>::Reporter>;

    /// The journey's data bag.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders").build()?;
    /// let instance = orders.instance(Orders).data("amount", 42_i64).create()?;
    /// assert_eq!(instance.data_bag().keys().collect::<Vec<_>>(), ["amount"]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    fn data_bag(&self) -> &DataBag;

    /// Gives up the instance for its data bag, once the journey has ended.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders").build()?;
    /// let instance = orders.instance(Orders).data("amount", 42_i64).create()?;
    /// assert!(instance.into_data_bag().get("amount").is_some());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    fn into_data_bag(self) -> DataBag;

    /// Commits a contribution to the data bag, replacing any value under its key, and says
    /// which it did.
    ///
    /// The executor calls it for each contribution of an attempt that succeeded, and turns the
    /// answer into events.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::{Committed, WorkflowInstance};
    /// use itinera::value::AnyValue;
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders").build()?;
    /// let mut instance = orders.instance(Orders).data("amount", 42_i64).create()?;
    /// let amount = instance.commit("amount".to_string(), AnyValue::new(40_i64));
    /// assert_eq!(amount, Committed::Overwritten);
    /// let receipt = instance.commit("receipt".to_string(), AnyValue::new("R-1".to_string()));
    /// assert_eq!(receipt, Committed::Added);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    fn commit(&mut self, key: String, value: AnyValue) -> Committed;
}

/// What committing a contribution did to the data bag.
///
/// # Examples
///
/// ```
/// use itinera::instance::Committed;
///
/// fn overwrote(committed: Committed) -> bool {
///     committed == Committed::Overwritten
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Committed {
    /// The data bag had no value under the key.
    Added,
    /// The value replaced the one the data bag had under the key.
    Overwritten,
}
