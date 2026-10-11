//! The listing: a workflow's steps in order, with their policies and input adapters, without
//! running anything.

use std::num::NonZeroUsize;

use super::input_adapter::attached_to;
use super::{AdapterName, InputAdapterDescriptor, WorkflowDescriptor};
use crate::mode::Mode;
use crate::policy::PolicyName;
use crate::step::{StepDescriptor, StepName};

impl<W: Send + Sync + 'static, M: Mode> WorkflowDescriptor<W, M> {
    /// Lists the workflow's steps in order, without running anything: each step's name, its
    /// position from 1, the names of its policies in the order they were attached, and the name of
    /// its input adapter, if it has one.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{
    ///     OnStepSuccess, OnSuccess, Requested, StepPolicyDescriptor, StepSuccess,
    /// };
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::{InputAdapterDescriptor, WorkflowDescriptor};
    ///
    /// struct Audit;
    ///
    /// impl<W: Send + Sync + 'static> OnStepSuccess<W> for Audit {
    ///     fn on_step_success(
    ///         &self,
    ///         _got: Requested<'_, W, StepSuccess>,
    ///     ) -> Result<Option<OnSuccess>, Error> {
    ///         Ok(None)
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let audit = StepPolicyDescriptor::new("audit", || Audit).on_step_success();
    /// let orders = WorkflowDescriptor::<Orders>::builder("orders")
    ///     .step(
    ///         StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())).policy(audit),
    ///     )
    ///     .step(StepDescriptor::new(step_name!("ship"), || Ok(Outcome::success())))
    ///     .input_adapter(InputAdapterDescriptor::new("pricing", step_name!("charge"), |_, _| Ok(None)))
    ///     .build()?;
    ///
    /// let listing = orders.listing();
    /// let charge = &listing[0];
    /// assert_eq!(charge.step, step_name!("charge"));
    /// assert_eq!(charge.position.get(), 1);
    /// assert_eq!(charge.policies.len(), 1);
    /// assert!(charge.adapter.is_some());
    /// assert!(listing[1].adapter.is_none());
    /// # Ok::<(), itinera::workflow::Violations>(())
    /// ```
    pub fn listing(&self) -> Vec<ListedStep> {
        let adapters = &self.declaration.adapters;
        self.declaration
            .steps
            .iter()
            .zip(positions())
            .map(|(step, position)| listed(step, position, adapters))
            .collect()
    }
}

/// One step of a workflow's listing.
///
/// # Examples
///
/// ```
/// use itinera::workflow::ListedStep;
///
/// fn describe(listed: &ListedStep) -> String {
///     format!("{}. {}", listed.position, listed.step)
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ListedStep {
    /// The step's name.
    pub step: StepName,
    /// Where the step runs among the workflow's steps, from 1.
    pub position: NonZeroUsize,
    /// The step policies attached to it, in the order they were attached.
    pub policies: Vec<PolicyName>,
    /// The input adapter attached to it, if any.
    pub adapter: Option<AdapterName>,
}

fn listed<M, W>(
    step: &StepDescriptor<W, M>,
    position: NonZeroUsize,
    adapters: &[InputAdapterDescriptor<W>],
) -> ListedStep {
    let name = step.name();
    ListedStep {
        step: name,
        position,
        policies: step.policies().iter().map(|policy| policy.name()).collect(),
        adapter: attached_to(adapters, name).map(InputAdapterDescriptor::<W>::name),
    }
}

/// The positions of steps: 1, 2, 3 and so on.
fn positions() -> impl Iterator<Item = NonZeroUsize> {
    std::iter::successors(Some(NonZeroUsize::MIN), next_position)
}

fn next_position(position: &NonZeroUsize) -> Option<NonZeroUsize> {
    position.checked_add(1)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::policy::StepPolicyDescriptor;
    use crate::policy::tests::Quiet;
    use crate::step::Outcome;

    struct Orders;

    #[test]
    fn a_workflow_is_listed_in_order_with_its_policies_and_adapters_without_running_anything() {
        let ran = Arc::new(AtomicUsize::new(0));
        let charged = Arc::clone(&ran);
        let shipped = Arc::clone(&ran);
        let orders = WorkflowDescriptor::<Orders>::builder("orders")
            .step(
                StepDescriptor::new(StepName::new("charge"), move || {
                    charged.fetch_add(1, Ordering::SeqCst);
                    Ok(Outcome::success())
                })
                .policy(StepPolicyDescriptor::new("audit", || Quiet).on_step_success())
                .policy(StepPolicyDescriptor::new("alarm", || Quiet).on_step_failure()),
            )
            .step(StepDescriptor::new(StepName::new("ship"), move || {
                shipped.fetch_add(1, Ordering::SeqCst);
                Ok(Outcome::success())
            }))
            .input_adapter(InputAdapterDescriptor::new(
                "pricing",
                StepName::new("charge"),
                |_, _| Ok(None),
            ))
            .build()
            .unwrap();

        assert_eq!(
            orders.listing(),
            [
                ListedStep {
                    step: StepName::new("charge"),
                    position: NonZeroUsize::new(1).unwrap(),
                    policies: vec![PolicyName::from("audit"), PolicyName::from("alarm")],
                    adapter: Some(AdapterName::from("pricing")),
                },
                ListedStep {
                    step: StepName::new("ship"),
                    position: NonZeroUsize::new(2).unwrap(),
                    policies: Vec::new(),
                    adapter: None,
                },
            ]
        );
        assert_eq!(ran.load(Ordering::SeqCst), 0);
    }
}
