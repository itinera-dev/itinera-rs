//! Creating an instance: its initial data and global metadata, then its journey ID and its
//! reporters.

use super::{Instance, InstanceError};
use crate::journey::{DataBag, InitialData};
use crate::mode::{Mode, Synchronous};
use crate::report::{GlobalMetadata, Reporter, held};
use crate::value::{AnyValue, Value};
use crate::workflow::WorkflowDescriptor;

#[cfg(feature = "async")]
mod asynchronous;
#[cfg(any(test, feature = "testing"))]
mod fake;

/// Creates an [`Instance`]: takes its initial data, its initial global metadata and the reporters
/// made for it alone, then produces its journey ID and makes the reporters the workflow lists.
/// The executor gives the journey ID and the global metadata to every reporter, before the
/// journey starts.
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
/// let instance = orders
///     .instance(Orders)
///     .data("amount", 42_i64)
///     .data("customer", "ana".to_string())
///     .create()?;
/// assert_eq!(instance.data_bag().keys().collect::<Vec<_>>(), ["amount", "customer"]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(derive_more::Debug)]
pub struct InstanceBuilder<W, M: Mode = Synchronous> {
    descriptor: WorkflowDescriptor<W, M>,
    #[debug(skip)]
    workflow: W,
    data: DataBag,
    global_metadata: GlobalMetadata,
    #[debug("{}", reporters.len())]
    reporters: Vec<M::Reporter>,
}

impl<W: Send + Sync + 'static, M: Mode> InstanceBuilder<W, M> {
    pub(crate) fn new(descriptor: WorkflowDescriptor<W, M>, workflow: W) -> Self {
        Self {
            descriptor,
            workflow,
            data: DataBag::new(),
            global_metadata: GlobalMetadata::new(),
            reporters: Vec::new(),
        }
    }

    /// Adds initial data under a key, replacing any value given before under the same key.
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
    /// let amount = instance.data_bag().get("amount").and_then(|v| v.downcast_ref::<i64>());
    /// assert_eq!(amount, Some(&42));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn data<T: Value>(mut self, key: impl Into<String>, value: T) -> Self {
        self.data.insert(key.into(), AnyValue::new(value));
        self
    }

    /// Adds initial data given as a type of the application's own, each of its keys replacing
    /// any value given before under the same key.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::journey::InitialData;
    /// use itinera::value::AnyValue;
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// struct NewOrder {
    ///     price: i64,
    /// }
    ///
    /// impl InitialData for NewOrder {
    ///     fn entries(self) -> impl IntoIterator<Item = (String, AnyValue)> {
    ///         [("price".to_owned(), AnyValue::new(self.price))]
    ///     }
    /// }
    ///
    /// let orders = WorkflowDescriptor::builder("orders").build()?;
    /// let instance = orders
    ///     .instance(Orders)
    ///     .initial_data(NewOrder { price: 42 })
    ///     .create()?;
    /// assert_eq!(instance.data_bag().keys().collect::<Vec<_>>(), ["price"]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn initial_data(mut self, data: impl InitialData) -> Self {
        for (key, value) in data.entries() {
            self.data.insert(key, value);
        }
        self
    }

    /// Adds an entry of initial global metadata, which every reporter receives before the
    /// journey starts, right after the journey ID. Setting a key again replaces its value, and
    /// the entries reach the reporters in the order their keys were first set. The key
    /// [`GlobalMetadata::JOURNEY_ID`] is the executor's own: setting it here makes `run` refuse
    /// the journey.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders").build()?;
    /// let instance = orders
    ///     .instance(Orders)
    ///     .reporters_global_metadata("tenant", "acme".to_string())
    ///     .create()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn reporters_global_metadata<T: Value>(mut self, key: impl Into<String>, value: T) -> Self {
        self.global_metadata.insert(key, AnyValue::new(value));
        self
    }

    /// Adds a reporter made for this journey alone, such as one that knows the request the
    /// journey serves. It receives events after the reporters the workflow lists, and after
    /// those added before it.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Result;
    /// use itinera::event::Event;
    /// use itinera::report::Reporter;
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// struct RequestLog {
    ///     request_id: String,
    /// }
    ///
    /// impl Reporter for RequestLog {
    ///     fn report(&mut self, event: &Event) -> Result<()> {
    ///         println!("[{}] {}", self.request_id, event.kind());
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let orders = WorkflowDescriptor::builder("orders").build()?;
    /// let request_log = RequestLog {
    ///     request_id: "req-7".to_string(),
    /// };
    /// let instance = orders.instance(Orders).reporter(request_log).create()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn reporter(mut self, reporter: impl Reporter) -> Self {
        self.reporters.push(held::<M>(reporter));
        self
    }

    /// Creates the instance: produces the journey ID, with the workflow's generator or as a UUID
    /// v4, then makes each reporter the workflow lists, which receive events before those made
    /// for this journey alone.
    ///
    /// # Errors
    ///
    /// [`InstanceError`] when the generator fails, or produces an empty journey ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::instance::InstanceError;
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .id_generator(|_: &Orders, _| Err(Error::msg("no number")))
    ///     .build()?;
    /// let error = orders.instance(Orders).create().unwrap_err();
    /// assert!(matches!(error, InstanceError::JourneyId(_)));
    /// # Ok::<(), itinera::workflow::Violations>(())
    /// ```
    pub fn create(self) -> Result<Instance<W, M>, InstanceError> {
        let Self {
            descriptor,
            workflow,
            data,
            global_metadata,
            reporters: made_for_it,
        } = self;
        let journey_id = descriptor.journey_id(&workflow, &data)?;
        let mut reporters = descriptor.reporters();
        reporters.extend(made_for_it);
        Ok(Instance {
            descriptor,
            workflow,
            journey_id,
            reporters,
            data,
            global_metadata,
        })
    }
}

#[cfg(test)]
mod tests;
