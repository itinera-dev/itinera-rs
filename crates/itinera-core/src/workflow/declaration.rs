//! A workflow's declaration, which its builder fills and its descriptor shares.

use super::{InputAdapterDescriptor, WorkflowName};
use crate::error::Error;
use crate::journey::DataBag;
use crate::mode::Mode;
use crate::policy::WorkflowPolicyEntry;
use crate::service::{ProviderEntry, Served};
use crate::step::StepDescriptor;
use crate::step::StepName;

#[derive(derive_more::Debug)]
pub(super) struct Declaration<W, M: Mode> {
    pub(super) name: WorkflowName,
    pub(super) steps: Vec<StepDescriptor<W, M>>,
    pub(super) policies: Vec<Box<dyn WorkflowPolicyEntry<W, M>>>,
    pub(super) adapters: Vec<InputAdapterDescriptor<W>>,
    pub(super) providers: Vec<Box<dyn ProviderEntry<W, M>>>,
    #[debug("{}", reporters.len())]
    pub(super) reporters: Vec<MakeReporter<M>>,
    #[debug("{}", id_generator.is_some())]
    pub(super) id_generator: Option<GenerateId<W>>,
}

impl<W, M: Mode> Declaration<W, M> {
    /// How a provider serves the step: as a step of the workflow, or, under a name the workflow
    /// declares no step under, as a rescue step.
    pub(super) fn served(&self, step: StepName) -> Served {
        if self.steps.iter().any(|declared| declared.is_named(step)) {
            Served::Step(step)
        } else {
            Served::RescueStep
        }
    }
}

/// Makes one listed reporter for a journey.
type MakeReporter<M> = fn() -> <M as Mode>::Reporter;

type GenerateId<W> = Box<dyn Fn(&W, &DataBag) -> Result<String, Error> + Send + Sync>;
