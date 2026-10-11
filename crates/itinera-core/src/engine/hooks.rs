//! Calling the hooks of a journey's policies: what a hook requests is resolved before it runs, and
//! what a policy's hook contributed is committed once it returns.

use super::end::End;
use super::request::Requesting;
use super::{Delivery, Journey};
use crate::error::Error;
use crate::event::{EventBody, HookSource, JourneyAbort, Source};
use crate::instance::WorkflowInstance;
use crate::journey::{Abort, Contributions, Contributor, DataBag};
use crate::policy::{
    Answers, Call, FailWorkflow, HookKind, Lifecycle, Needs, OnSuccess, PolicyName, Request,
    Requested,
};
use crate::step::Reporting;

mod step;
mod workflow;

pub(crate) use workflow::WorkflowPolicies;

use step::Left;

/// A lifecycle a policy's hook returned, which decides what happens next.
#[derive(Debug)]
pub(super) struct Decided<L> {
    pub(super) policy: PolicyName,
    pub(super) lifecycle: L,
}

/// What a hook returned, as `hook_called` reports it.
trait Returned {
    fn lifecycle(&self) -> Option<Lifecycle>;
}

impl Returned for Option<OnSuccess> {
    fn lifecycle(&self) -> Option<Lifecycle> {
        self.clone().map(Lifecycle::from)
    }
}

impl Returned for Option<FailWorkflow> {
    fn lifecycle(&self) -> Option<Lifecycle> {
        self.clone().map(Lifecycle::from)
    }
}

impl Returned for () {
    fn lifecycle(&self) -> Option<Lifecycle> {
        None
    }
}

/// One call of a hook: which hook it is, what it needs, what the executor tells it, and what
/// its step's attempt left for it to request.
struct Called<'s, H: HookKind> {
    hook: HookSource,
    needs: &'s Needs,
    context: H::Context,
    left: Option<Left<'s>>,
}

impl<D: Delivery> Journey<D> {
    /// Resolves what the hook requests, calls it, and once it returns without failing, reports
    /// what it returned and commits what it contributed.
    async fn call_hook<I: WorkflowInstance, P: ?Sized + Sync, H: HookKind, R: Returned>(
        &mut self,
        instance: &mut I,
        policy: &P,
        call: Call<P, I::Workflow, H, I::Mode, R>,
        called: Called<'_, H>,
    ) -> Result<R, End> {
        let Called {
            hook,
            needs,
            context,
            left,
        } = called;
        let requesting = Requesting::hook(&hook);
        let data_bag = instance.data_bag();
        let mut answers = Answers::default();
        for request in needs.requests() {
            self.answer(&mut answers, &requesting, request, data_bag, left.as_ref())
                .await?;
        }
        let mut contributions = Contributions::default();
        let returned = {
            let contributor = if needs.wants_contributor() {
                Some(Contributor::new(&mut contributions))
            } else {
                None
            };
            let reporting = if needs.wants_reporter() {
                Some(Reporting::new(self, hook.clone()))
            } else {
                None
            };
            answers.contributor = contributor;
            answers.reporting = reporting;
            let got = Requested::new(instance.workflow(), instance.journey_id(), context, answers);
            call(policy, got).await
        };
        if let Some(aborted) = self.interrupted.take() {
            return Err(End::Aborted(aborted));
        }
        let returned = match returned {
            Ok(returned) => returned,
            Err(error) => return Err(hook_failed(&hook, error)),
        };
        self.emit(EventBody::HookCalled {
            hook: hook.clone(),
            lifecycle: returned.lifecycle(),
        })
        .await?;
        self.commit(instance, Source::Hook(hook), &contributions)
            .await?;
        Ok(returned)
    }

    /// Resolves one of a hook's requests into its answers, or into the abort it causes.
    pub(super) async fn answer<'s>(
        &mut self,
        answers: &mut Answers<'s>,
        requesting: &Requesting,
        request: &Request,
        data_bag: &DataBag,
        left: Option<&Left<'s>>,
    ) -> Result<(), End> {
        match request {
            Request::FromStep(need) => {
                let key = need.key();
                let found = left.and_then(|left| left.contributed(key));
                let value = self.request(requesting, need, found).await?;
                answers.from_step.hold(key, value);
            }
            Request::FromWorkflow(need) => {
                let key = need.key();
                let value = self
                    .request(requesting, need, data_bag.get(key).cloned())
                    .await?;
                answers.from_workflow.hold(key, value);
            }
            Request::Reason(requirement) => {
                answers.reason = left.map(|left| left.reason(*requirement)).transpose()?;
            }
            Request::Error(requirement) => {
                answers.error = left.map(|left| left.error(*requirement)).transpose()?;
            }
        }
        Ok(())
    }
}

fn hook_failed(hook: &HookSource, error: Error) -> End {
    let step = match hook {
        HookSource::Step { step, .. } => Some(step.step),
        HookSource::Workflow { .. } => None,
    };
    let reported = JourneyAbort::HookFailed {
        step,
        error: error.to_string(),
    };
    End::aborted(Abort::HookFailed(error), reported)
}

#[cfg(test)]
mod tests;
