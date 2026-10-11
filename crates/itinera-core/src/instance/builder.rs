//! Creating an instance: its initial data, then its journey ID and its reporters.

use super::Instance;
use crate::error::Error;
use crate::journey::DataBag;
use crate::mode::{Mode, Synchronous};
use crate::value::{AnyValue, Value};
use crate::workflow::WorkflowDescriptor;

/// Creates an [`Instance`]: takes its initial data, then produces its journey ID and makes its
/// reporters, in that order.
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
}

impl<W: Send + Sync + 'static, M: Mode> InstanceBuilder<W, M> {
    pub(crate) fn new(descriptor: WorkflowDescriptor<W, M>, workflow: W) -> Self {
        Self {
            descriptor,
            workflow,
            data: DataBag::new(),
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

    /// Creates the instance: produces its journey ID, with the workflow's generator or as a
    /// UUID v4, then makes each of the workflow's reporters for the journey.
    ///
    /// # Errors
    ///
    /// [`InstanceError`] when the generator or a reporter's `init` fails.
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
        } = self;
        let journey_id = descriptor
            .journey_id(&workflow, &data)
            .map_err(InstanceError::JourneyId)?;
        let reporters = descriptor
            .reporters(&workflow, &journey_id, &data)
            .map_err(InstanceError::Reporter)?;
        Ok(Instance {
            descriptor,
            workflow,
            journey_id,
            reporters,
            data,
        })
    }
}

/// Why an instance could not be created. No journey exists yet, so nothing is reported.
///
/// # Examples
///
/// ```
/// use itinera::instance::InstanceError;
///
/// fn describe(error: &InstanceError) -> String {
///     error.to_string()
/// }
/// ```
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum InstanceError {
    /// The workflow's generator failed to produce the journey ID.
    #[error("the journey ID could not be produced: {0}")]
    JourneyId(Error),
    /// A reporter's `init` failed.
    #[error("a reporter could not be made: {0}")]
    Reporter(Error),
}

#[cfg(test)]
mod tests;
