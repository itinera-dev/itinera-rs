//! Workflow instances: one per journey, holding its journey ID, its reporters and its data.

use crate::journey::{DataBag, JourneyId};
use crate::mode::{Mode, Synchronous};
use crate::value::AnyValue;
use crate::workflow::WorkflowDescriptor;

mod builder;
#[cfg(test)]
mod fixtures;
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
    use super::*;
    use crate::instance::fixtures::{Audit, Orders, orders};

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
