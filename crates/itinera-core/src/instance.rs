//! Workflow instances: one per journey, holding its journey ID, its reporters and its data.

use crate::journey::{DataBag, JourneyId};
use crate::mode::{Mode, Synchronous};
use crate::report::GlobalMetadata;
use crate::value::AnyValue;
use crate::workflow::WorkflowDescriptor;

mod builder;
mod committed;
mod error;
#[cfg(test)]
mod fixtures;
mod workflow_instance;

pub use builder::InstanceBuilder;
pub use committed::Committed;
pub use error::InstanceError;
pub use workflow_instance::WorkflowInstance;

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
    global_metadata: GlobalMetadata,
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

    fn take_global_metadata(&mut self) -> GlobalMetadata {
        std::mem::take(&mut self.global_metadata)
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
    use super::*;
    use crate::instance::fixtures::{Audit, Orders, orders};

    #[test]
    fn an_instance_hands_its_initial_global_metadata_over_once() {
        let orders = orders().build().unwrap();
        let mut instance = orders
            .instance(Orders::new())
            .global_metadata("tenant", "acme".to_string())
            .create()
            .unwrap();

        let tenant = instance.take_global_metadata();
        assert_eq!(
            tenant
                .get("tenant")
                .and_then(AnyValue::downcast_ref::<String>),
            Some(&"acme".to_string())
        );
        assert!(instance.take_global_metadata().get("tenant").is_none());
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

        assert_eq!(instance.workflow().shop, "north");
        let amount = instance.into_data_bag().get("amount").cloned();
        assert_eq!(
            amount
                .as_ref()
                .and_then(|value| value.downcast_ref::<i64>()),
            Some(&43)
        );
    }

    #[test]
    fn an_instance_shows_its_journey_its_data_its_global_metadata_and_how_many_reporters_it_has_but_not_its_workflow()
     {
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
             providers: [], reporters: 1, id_generator: true } }, \
             journey_id: JourneyId(\"order-7\"), reporters: 1, \
             data: DataBag { values: {\"amount\": AnyValue(\"i64\")} }, \
             global_metadata: GlobalMetadata { entries: [] }, .. }"
        );
    }
}
