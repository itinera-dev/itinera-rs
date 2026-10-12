//! The listing: a workflow's steps in order, with their policies, input adapters, services and
//! rescue steps, without running anything.

use std::num::NonZeroUsize;

use super::{AdapterName, InputAdapterDescriptor, WorkflowDescriptor};
use crate::mode::Mode;
use crate::service::ProviderEntry;
use crate::step::ServiceNeed;
use crate::step::{StepDescriptor, StepName};
use listed_service::served_by;

mod listed_rescue;
mod listed_service;
mod listed_step;

pub use listed_rescue::ListedRescue;
pub use listed_service::ListedService;
pub use listed_step::ListedStep;

impl<W: Send + Sync + 'static, M: Mode> WorkflowDescriptor<W, M> {
    /// Lists the workflow's steps in order, without running anything: each step's name, its
    /// position from 1, the names of its policies in the order they were attached, the name of
    /// its input adapter, if it has one, and the services it requests, each with the provider
    /// that serves it. A step with a rescue step also has a rescue section, which lists the rescue
    /// step's name, its input adapter and its services in the same way.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{
    ///     OnStepSuccess, OnSuccess, Requested, StepPolicyDescriptor, StepSuccess,
    /// };
    /// use itinera::step::{Outcome, RescueOutcome, StepDescriptor, step_name};
    /// use itinera::workflow::{AdapterName, InputAdapterDescriptor, WorkflowDescriptor};
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
    ///         StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success()))
    ///             .policy(audit)
    ///             .rescue(|| Ok(RescueOutcome::success())),
    ///     )
    ///     .step(StepDescriptor::new(step_name!("ship"), || Ok(Outcome::success())))
    ///     .input_adapter(InputAdapterDescriptor::new("pricing", step_name!("charge"), |_, _| Ok(None)))
    ///     .input_adapter(InputAdapterDescriptor::new(
    ///         "credentials",
    ///         step_name!("charge_rescue"),
    ///         |_, _| Ok(None),
    ///     ))
    ///     .build()?;
    ///
    /// let listing = orders.listing();
    /// let charge = &listing[0];
    /// assert_eq!(charge.step, step_name!("charge"));
    /// assert_eq!(charge.position.get(), 1);
    /// assert_eq!(charge.policies.len(), 1);
    /// assert!(charge.adapter.is_some());
    /// let rescue = charge.rescue.as_ref().ok_or("charge has a rescue step")?;
    /// assert_eq!(rescue.step, step_name!("charge_rescue"));
    /// assert_eq!(rescue.adapter, Some(AdapterName::from("credentials")));
    /// assert!(listing[1].adapter.is_none());
    /// assert!(listing[1].rescue.is_none());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn listing(&self) -> Vec<ListedStep> {
        self.declaration
            .steps
            .iter()
            .zip(positions())
            .map(|(step, position)| self.listed(step, position))
            .collect()
    }

    fn listed(&self, step: &StepDescriptor<W, M>, position: NonZeroUsize) -> ListedStep {
        ListedStep {
            step: step.name(),
            position,
            policies: step.policies().iter().map(|policy| policy.name()).collect(),
            adapter: self.adapter_of(step),
            services: self.services_of(step),
            rescue: step.rescue_step().map(|rescue| self.rescue_section(rescue)),
        }
    }

    fn rescue_section(&self, rescue: &StepDescriptor<W, M>) -> ListedRescue {
        ListedRescue {
            step: rescue.name(),
            adapter: self.adapter_of(rescue),
            services: self.services_of(rescue),
        }
    }

    fn adapter_of(&self, step: &StepDescriptor<W, M>) -> Option<AdapterName> {
        self.adapter(step.name())
            .map(InputAdapterDescriptor::<W>::name)
    }

    fn services_of(&self, step: &StepDescriptor<W, M>) -> Vec<ListedService> {
        let name = step.name();
        step.needs()
            .services()
            .iter()
            .filter_map(|need| self.listed_service(name, need))
            .collect()
    }

    /// A service the step requests, with the provider that serves it, which admission made sure
    /// there is.
    fn listed_service(&self, step: StepName, need: &ServiceNeed) -> Option<ListedService> {
        let key = need.key();
        self.provider(step, key)
            .map(ProviderEntry::name)
            .map(|provider| served_by(key, provider))
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
    use crate::policy::fixtures::Quiet;
    use crate::policy::{PolicyName, StepPolicyDescriptor};
    use crate::service::ProviderName;
    use crate::step::{Outcome, RescueOutcome};

    struct Orders;

    #[test]
    fn a_workflow_is_listed_in_order_without_running_anything() {
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
                .policy(StepPolicyDescriptor::new("alarm", || Quiet).on_step_failure())
                .rescue(|| Ok(RescueOutcome::success())),
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
            .input_adapter(InputAdapterDescriptor::new(
                "credentials",
                StepName::new("charge_rescue"),
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
                    services: Vec::new(),
                    rescue: Some(ListedRescue {
                        step: StepName::new("charge_rescue"),
                        adapter: Some(AdapterName::from("credentials")),
                        services: Vec::new(),
                    }),
                },
                ListedStep {
                    step: StepName::new("ship"),
                    position: NonZeroUsize::new(2).unwrap(),
                    policies: Vec::new(),
                    adapter: None,
                    services: Vec::new(),
                    rescue: None,
                },
            ]
        );
        assert_eq!(ran.load(Ordering::SeqCst), 0);
    }

    mod services {
        use super::*;
        use crate::service::fixtures::{
            Given, Made, PAYMENTS, STOCK, per_attempt, per_journey, requesting, rescued_requesting,
        };
        use crate::workflow::fixtures::orders;

        #[test]
        fn each_step_is_listed_with_the_services_it_requests_and_the_provider_serving_each() {
            let (made, given) = (Made::default(), Given::default());
            let orders = orders()
                .step(requesting("charge", &[&STOCK, &PAYMENTS], &[], &given))
                .step(requesting("ship", &[&STOCK], &[], &given))
                .provider(per_journey("stock", &STOCK, &made))
                .provider(per_attempt("charging", &PAYMENTS, &made).step(StepName::new("charge")))
                .build()
                .unwrap();

            let services: Vec<Vec<ListedService>> =
                orders.listing().into_iter().map(services_of).collect();

            assert_eq!(
                services,
                [
                    vec![
                        served_by("stock", ProviderName::from("stock")),
                        served_by("payments", ProviderName::from("charging")),
                    ],
                    vec![served_by("stock", ProviderName::from("stock"))],
                ]
            );
        }

        #[test]
        fn a_rescue_section_lists_the_services_its_rescue_step_requests_and_the_provider_serving_each()
         {
            let (made, given) = (Made::default(), Given::default());
            let charge = requesting("charge", &[&PAYMENTS], &[], &given);
            let orders = orders()
                .step(rescued_requesting(charge, &[&PAYMENTS, &STOCK], &given))
                .provider(per_journey("payments", &PAYMENTS, &made))
                .provider(per_journey("stock", &STOCK, &made))
                .provider(
                    per_attempt("backup", &PAYMENTS, &made).step(StepName::new("charge_rescue")),
                )
                .build()
                .unwrap();

            let rescues: Vec<Option<ListedRescue>> =
                orders.listing().into_iter().map(rescue_of).collect();

            assert_eq!(
                rescues,
                [Some(ListedRescue {
                    step: StepName::new("charge_rescue"),
                    adapter: None,
                    services: vec![
                        served_by("payments", ProviderName::from("backup")),
                        served_by("stock", ProviderName::from("stock")),
                    ],
                })]
            );
        }

        fn services_of(listed: ListedStep) -> Vec<ListedService> {
            listed.services
        }

        fn rescue_of(listed: ListedStep) -> Option<ListedRescue> {
            listed.rescue
        }
    }
}
