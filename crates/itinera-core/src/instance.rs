//! Workflow instances: one per journey, holding its journey ID, its reporters and its data.

use crate::journey::{DataBag, JourneyId};
use crate::mode::{Mode, Synchronous};
use crate::value::AnyValue;
use crate::workflow::WorkflowDescriptor;

mod builder;
mod workflow_instance;

pub use builder::{InstanceBuilder, InstanceError};
pub use workflow_instance::{Committed, WorkflowInstance};

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
/// let orders = WorkflowDescriptor::builder("orders").build()?;
/// let instance: Instance<Orders> = orders.instance(Orders).create()?;
/// assert_eq!(instance.descriptor().name().to_string(), "orders");
/// # Ok::<(), Box<dyn std::error::Error>>(())
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

    fn commit(&mut self, key: String, value: AnyValue) -> Committed {
        match self.data.insert(key, value) {
            Some(_) => Committed::Overwritten,
            None => Committed::Added,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::error::Error;
    use crate::event::Event;
    use crate::report::{Reporter, WorkflowReporter};
    use crate::workflow::WorkflowBuilder;

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

    fn orders() -> WorkflowBuilder<Orders> {
        WorkflowDescriptor::builder("orders")
    }

    fn numbered_orders() -> WorkflowBuilder<Orders> {
        orders().id_generator(numbered)
    }

    fn is_uuid_v4(id: &str) -> bool {
        uuid::Uuid::parse_str(id).is_ok_and(is_version_4)
    }

    fn is_version_4(uuid: uuid::Uuid) -> bool {
        uuid.get_version_num() == 4
    }

    #[test]
    fn without_a_generator_the_journey_id_is_a_uuid_v4() {
        let orders = orders().build().unwrap();
        let first = orders.instance(Orders::new()).create().unwrap();
        let second = orders.instance(Orders::new()).create().unwrap();

        assert!(is_uuid_v4(first.journey_id().as_ref()));
        assert_ne!(first.journey_id(), second.journey_id());
    }

    #[test]
    fn the_generator_produces_the_journey_id_from_the_initial_data() {
        let orders = numbered_orders().build().unwrap();
        let instance = orders
            .instance(Orders::new())
            .data("number", 7_i64)
            .create()
            .unwrap();

        assert_eq!(instance.journey_id().to_string(), "order-7");
    }

    #[test]
    fn a_generator_that_fails_makes_creating_the_instance_fail() {
        let orders = numbered_orders().build().unwrap();
        let error = orders.instance(Orders::new()).create().unwrap_err();

        assert!(matches!(error, InstanceError::JourneyId(_)));
        assert_eq!(
            error.to_string(),
            "the journey ID could not be produced: no number"
        );
    }

    #[test]
    fn reporters_are_made_after_the_journey_id_with_the_initial_data() {
        let orders = numbered_orders()
            .reporter::<Audit>()
            .reporter::<Audit>()
            .build()
            .unwrap();
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
        let orders = orders()
            .reporter::<Audit>()
            .reporter::<Broken>()
            .build()
            .unwrap();
        let error = orders.instance(Orders::new()).create().unwrap_err();

        assert!(matches!(error, InstanceError::Reporter(_)));
        assert_eq!(
            error.to_string(),
            "a reporter could not be made: the audit log is closed"
        );
    }

    #[test]
    fn the_instance_holds_the_workflows_value_and_its_initial_data() {
        let orders = orders().build().unwrap();
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
        fn an_asynchronous_workflow_makes_reporters_of_both_kinds_in_order() {
            let orders: WorkflowDescriptor<Orders, Asynchronous> =
                WorkflowDescriptor::async_builder("orders")
                    .reporter::<Audit>()
                    .async_reporter::<Forward>()
                    .reporter::<Audit>()
                    .build()
                    .unwrap();
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
        let workflow = orders()
            .reporter::<Audit>()
            .id_generator(|_: &Orders, _| Ok("order-7".to_string()))
            .build()
            .unwrap();
        let instance = workflow
            .instance(Orders::new())
            .data("amount", 42_i64)
            .create()
            .unwrap();
        assert_eq!(
            format!("{instance:?}"),
            "Instance { \
             descriptor: WorkflowDescriptor { declaration: Declaration { \
             name: WorkflowName(\"orders\"), steps: [], policies: [], adapters: [], \
             reporters: 1, id_generator: true } }, \
             journey_id: JourneyId(\"order-7\"), reporters: 1, \
             data: DataBag { values: {\"amount\": AnyValue(\"i64\")} }, .. }"
        );
    }
}
