//! Workflow instances: one per journey, holding its journey ID, its reporters and its data.

use crate::error::Error;
use crate::journey::{DataBag, JourneyId};
use crate::mode::{Mode, Synchronous};
use crate::value::{AnyValue, Value};
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
    ///     .build();
    /// let instance = orders.instance(Orders).create()?;
    /// assert_eq!(instance.journey_id().to_string(), "order-1");
    /// # Ok::<(), itinera::instance::InstanceError>(())
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
    /// let orders = WorkflowDescriptor::builder("orders").build();
    /// let mut instance = orders.instance(Orders).create()?;
    /// assert!(instance.take_reporters().is_empty());
    /// # Ok::<(), itinera::instance::InstanceError>(())
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
    /// let orders = WorkflowDescriptor::builder("orders").build();
    /// let instance = orders.instance(Orders).data("amount", 42_i64).create()?;
    /// assert_eq!(instance.data_bag().keys().collect::<Vec<_>>(), ["amount"]);
    /// # Ok::<(), itinera::instance::InstanceError>(())
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
    /// let orders = WorkflowDescriptor::builder("orders").build();
    /// let instance = orders.instance(Orders).data("amount", 42_i64).create()?;
    /// assert!(instance.into_data_bag().get("amount").is_some());
    /// # Ok::<(), itinera::instance::InstanceError>(())
    /// ```
    fn into_data_bag(self) -> DataBag;
}

/// The workflow instance itinera makes, with [`WorkflowDescriptor::instance`].
///
/// # Examples
///
/// ```
/// use itinera::instance::{Instance, WorkflowInstance};
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Orders;
///
/// let orders = WorkflowDescriptor::builder("orders").build();
/// let instance: Instance<Orders> = orders.instance(Orders).create()?;
/// assert_eq!(instance.descriptor().name().to_string(), "orders");
/// # Ok::<(), itinera::instance::InstanceError>(())
/// ```
#[derive(derive_more::Debug)]
pub struct Instance<W, M: Mode = Synchronous> {
    descriptor: WorkflowDescriptor<W, M>,
    #[debug(skip)]
    workflow: W,
    journey_id: JourneyId,
    #[debug("{}", reporters.len())]
    reporters: Vec<M::Reporter>,
    data: DataBag,
}

impl<W: Send + Sync + 'static, M: Mode> WorkflowInstance for Instance<W, M> {
    type Workflow = W;
    type Mode = M;

    fn descriptor(&self) -> &WorkflowDescriptor<W, M> {
        &self.descriptor
    }

    fn workflow(&self) -> &W {
        &self.workflow
    }

    fn journey_id(&self) -> &JourneyId {
        &self.journey_id
    }

    fn take_reporters(&mut self) -> Vec<M::Reporter> {
        std::mem::take(&mut self.reporters)
    }

    fn data_bag(&self) -> &DataBag {
        &self.data
    }

    fn into_data_bag(self) -> DataBag {
        self.data
    }
}

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
/// let orders = WorkflowDescriptor::builder("orders").build();
/// let instance = orders
///     .instance(Orders)
///     .data("amount", 42_i64)
///     .data("customer", "ana".to_string())
///     .create()?;
/// assert_eq!(instance.data_bag().keys().collect::<Vec<_>>(), ["amount", "customer"]);
/// # Ok::<(), itinera::instance::InstanceError>(())
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
    /// let orders = WorkflowDescriptor::builder("orders").build();
    /// let instance = orders.instance(Orders).data("amount", 42_i64).create()?;
    /// let amount = instance.data_bag().get("amount").and_then(|v| v.downcast_ref::<i64>());
    /// assert_eq!(amount, Some(&42));
    /// # Ok::<(), itinera::instance::InstanceError>(())
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
    ///     .build();
    /// let error = orders.instance(Orders).create().unwrap_err();
    /// assert!(matches!(error, InstanceError::JourneyId(_)));
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
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::event::Event;
    use crate::report::{Reporter, WorkflowReporter};

    struct Orders {
        made: Arc<Mutex<Vec<String>>>,
    }

    impl Orders {
        fn new() -> Self {
            Self {
                made: Arc::default(),
            }
        }

        fn made(&self) -> Vec<String> {
            self.made.lock().unwrap().clone()
        }
    }

    struct Audit;

    impl Reporter for Audit {
        fn report(&mut self, _event: &Event) -> Result<(), Error> {
            Ok(())
        }
    }

    impl WorkflowReporter<Orders> for Audit {
        fn init(orders: &Orders, journey_id: &JourneyId, data: &DataBag) -> Result<Self, Error> {
            let keys: Vec<&str> = data.keys().collect();
            orders
                .made
                .lock()
                .unwrap()
                .push(format!("audit for {journey_id} with {keys:?}"));
            Ok(Audit)
        }
    }

    struct Broken;

    impl Reporter for Broken {
        fn report(&mut self, _event: &Event) -> Result<(), Error> {
            Ok(())
        }
    }

    impl WorkflowReporter<Orders> for Broken {
        fn init(_: &Orders, _: &JourneyId, _: &DataBag) -> Result<Self, Error> {
            Err(Error::msg("the audit log is closed"))
        }
    }

    fn numbered(_: &Orders, data: &DataBag) -> Result<String, Error> {
        let number = data
            .get("number")
            .and_then(|value| value.downcast_ref::<i64>())
            .ok_or_else(|| Error::msg("no number"))?;
        Ok(format!("order-{number}"))
    }

    fn is_uuid_v4(id: &str) -> bool {
        uuid::Uuid::parse_str(id).is_ok_and(is_version_4)
    }

    fn is_version_4(uuid: uuid::Uuid) -> bool {
        uuid.get_version_num() == 4
    }

    #[test]
    fn without_a_generator_the_journey_id_is_a_uuid_v4() {
        let orders = WorkflowDescriptor::builder("orders").build();
        let first = orders.instance(Orders::new()).create().unwrap();
        let second = orders.instance(Orders::new()).create().unwrap();

        assert!(is_uuid_v4(first.journey_id().as_ref()));
        assert_ne!(first.journey_id(), second.journey_id());
    }

    #[test]
    fn the_generator_produces_the_journey_id_from_the_initial_data() {
        let orders = WorkflowDescriptor::builder("orders")
            .id_generator(numbered)
            .build();
        let instance = orders
            .instance(Orders::new())
            .data("number", 7_i64)
            .create()
            .unwrap();

        assert_eq!(instance.journey_id().to_string(), "order-7");
    }

    #[test]
    fn a_generator_that_fails_makes_creating_the_instance_fail() {
        let orders = WorkflowDescriptor::builder("orders")
            .id_generator(numbered)
            .build();
        let error = orders.instance(Orders::new()).create().unwrap_err();

        assert!(matches!(error, InstanceError::JourneyId(_)));
        assert_eq!(
            error.to_string(),
            "the journey ID could not be produced: no number"
        );
    }

    #[test]
    fn reporters_are_made_after_the_journey_id_with_the_initial_data() {
        let orders = WorkflowDescriptor::builder("orders")
            .id_generator(numbered)
            .reporter::<Audit>()
            .reporter::<Audit>()
            .build();
        let workflow = Orders::new();
        let made = Arc::clone(&workflow.made);
        let mut instance = orders
            .instance(workflow)
            .data("number", 7_i64)
            .create()
            .unwrap();

        assert_eq!(instance.take_reporters().len(), 2);
        assert!(instance.take_reporters().is_empty());
        assert_eq!(
            *made.lock().unwrap(),
            [
                r#"audit for order-7 with ["number"]"#,
                r#"audit for order-7 with ["number"]"#,
            ]
        );
    }

    #[test]
    fn a_reporter_that_cannot_be_made_makes_creating_the_instance_fail() {
        let orders = WorkflowDescriptor::builder("orders")
            .reporter::<Audit>()
            .reporter::<Broken>()
            .build();
        let error = orders.instance(Orders::new()).create().unwrap_err();

        assert!(matches!(error, InstanceError::Reporter(_)));
        assert_eq!(
            error.to_string(),
            "a reporter could not be made: the audit log is closed"
        );
    }

    #[test]
    fn the_instance_holds_the_workflows_value_and_its_initial_data() {
        let orders = WorkflowDescriptor::builder("orders").build();
        let instance = orders
            .instance(Orders::new())
            .data("amount", 42_i64)
            .data("amount", 43_i64)
            .create()
            .unwrap();

        assert!(instance.workflow().made().is_empty());
        let amount = instance.into_data_bag().get("amount").cloned();
        assert_eq!(
            amount
                .as_ref()
                .and_then(|value| value.downcast_ref::<i64>()),
            Some(&43)
        );
    }

    #[cfg(feature = "async")]
    mod asynchronous {
        use super::*;
        use crate::mode::Asynchronous;
        use crate::report::{AsyncReporter, AsyncWorkflowReporter, BoxedReporter};

        struct Forward;

        impl AsyncReporter for Forward {
            async fn report(&mut self, _event: &Event) -> Result<(), Error> {
                Ok(())
            }
        }

        impl AsyncWorkflowReporter<Orders> for Forward {
            fn init(orders: &Orders, _: &JourneyId, _: &DataBag) -> Result<Self, Error> {
                orders.made.lock().unwrap().push("forward".to_string());
                Ok(Forward)
            }
        }

        fn kind(reporter: &BoxedReporter) -> String {
            format!("{reporter:?}")
        }

        #[test]
        fn an_asynchronous_reporter_makes_the_workflow_asynchronous_and_keeps_the_order() {
            let orders: WorkflowDescriptor<Orders, Asynchronous> =
                WorkflowDescriptor::builder("orders")
                    .reporter::<Audit>()
                    .async_reporter::<Forward>()
                    .reporter::<Audit>()
                    .build();
            let workflow = Orders::new();
            let made = Arc::clone(&workflow.made);
            let mut instance = orders.instance(workflow).create().unwrap();

            let kinds: Vec<String> = instance.take_reporters().iter().map(kind).collect();
            assert_eq!(
                kinds,
                [
                    r#"BoxedReporter("sync")"#,
                    r#"BoxedReporter("async")"#,
                    r#"BoxedReporter("sync")"#,
                ]
            );
            assert_eq!(made.lock().unwrap()[1], "forward");
            assert_eq!(made.lock().unwrap().len(), 3);
        }
    }

    #[test]
    fn an_instance_shows_its_journey_its_data_and_how_many_reporters_it_has_but_not_its_workflow() {
        let workflow = WorkflowDescriptor::builder("orders")
            .reporter::<Audit>()
            .id_generator(|_: &Orders, _| Ok("order-7".to_string()))
            .build();
        let instance = workflow
            .instance(Orders::new())
            .data("amount", 42_i64)
            .create()
            .unwrap();
        assert_eq!(
            format!("{instance:?}"),
            "Instance { \
             descriptor: WorkflowDescriptor { declaration: Declaration { \
             name: WorkflowName(\"orders\"), step: None, reporters: 1, id_generator: true } }, \
             journey_id: JourneyId(\"order-7\"), reporters: 1, \
             data: DataBag { values: {\"amount\": AnyValue(\"i64\")} }, .. }"
        );
    }
}
