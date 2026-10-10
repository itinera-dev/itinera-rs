//! Calling the hooks of a journey's policies and the input adapters of its workflow: what a hook
//! requests is resolved before it runs, and what a policy's hook contributed is committed once it
//! returns.

use super::decision::Failed;
use super::{Attempting, Delivery, End, Journey, Requesting, Sources, could_not_build, wrong_type};
use crate::error::Error;
use crate::event::{self, EventBody, HookSource, JourneyAbort, Source};
use crate::instance::WorkflowInstance;
use crate::journey::{Abort, Contributions, Contributor, DataBag, MissingData};
use crate::policy::{
    Answers, BuiltStepPolicy, BuiltWorkflowPolicy, Call, FailWorkflow, HookKind, Lifecycle, Needs,
    OnSuccess, PolicyName, Request, Requested, RetryCause, StepAbnormalTermination, StepFailure,
    StepFailureCause, StepHook, StepHookKind, StepRetry, StepSuccess, WorkflowFailure,
    WorkflowHook, WorkflowSuccess,
};
use crate::step::{InputNeed, Reason, Reporting, Requirement, StepAttempt, StepName};
use crate::value::AnyValue;
use crate::workflow::{InputAdapterDescriptor, WorkflowDescriptor};

/// The workflow policies built for one journey, in the order they were attached.
pub(crate) type WorkflowPolicies<W, M> = Vec<Box<dyn BuiltWorkflowPolicy<W, M>>>;

/// How the engine calls one step hook of kind `H` on a built policy, which returns `R`.
type StepHookCall<W, M, H, R> = Call<dyn BuiltStepPolicy<W, M>, W, H, M, R>;

/// How the engine calls one workflow hook of kind `H` on a built policy.
type WorkflowHookCall<W, M, H> = Call<dyn BuiltWorkflowPolicy<W, M>, W, H, M, ()>;

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

/// What a step's attempt left for a step hook to request, and which hook requests it.
struct Left<'s> {
    policy: PolicyName,
    hook: StepHook,
    step: StepName,
    /// What the attempt contributed, committed or not.
    contributed: &'s Contributions,
    /// How the attempt failed, if it did.
    failed: Option<&'s Failed>,
}

impl<'s> Left<'s> {
    fn contributed(&self, key: &str) -> Option<AnyValue> {
        self.contributed.get(key).cloned()
    }

    /// The attempt's reason, for a hook that requested it as `requirement`.
    fn reason(&self, requirement: Requirement) -> Result<Option<Reason>, End> {
        match (requirement, self.failed.and_then(Failed::reason)) {
            (Requirement::Required, None) => Err(missing_data(
                MissingData::Reason {
                    policy: self.policy,
                    hook: self.hook,
                },
                event::MissingData::Reason {
                    policy: self.policy,
                    hook: self.hook,
                    step: self.step,
                },
            )),
            (_, found) => Ok(found.cloned()),
        }
    }

    /// The attempt's error, for a hook that requested it as `requirement`.
    fn error(&self, requirement: Requirement) -> Result<Option<&'s Error>, End> {
        match (requirement, self.failed.and_then(Failed::error)) {
            (Requirement::Required, None) => Err(missing_data(
                MissingData::Error {
                    policy: self.policy,
                    hook: self.hook,
                },
                event::MissingData::Error {
                    policy: self.policy,
                    hook: self.hook,
                    step: self.step,
                },
            )),
            (_, found) => Ok(found),
        }
    }
}

impl<D: Delivery> Journey<D> {
    /// `on step success`, after an attempt that succeeded, once its contributions are committed.
    pub(super) async fn on_step_success<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        attempting: &Attempting<'_, I::Workflow, I::Mode>,
    ) -> Result<Option<Decided<OnSuccess>>, End> {
        self.step_hook::<I, StepSuccess, _>(
            instance,
            attempting,
            StepHook::OnStepSuccess,
            <dyn BuiltStepPolicy<I::Workflow, I::Mode>>::on_step_success,
            attempting.attempt.clone(),
            None,
        )
        .await
    }

    /// `on step failure`, after the step was given up for `cause`.
    pub(super) async fn on_step_failure<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        attempting: &Attempting<'_, I::Workflow, I::Mode>,
        failed: &Failed,
        cause: StepFailureCause,
    ) -> Result<Option<Decided<FailWorkflow>>, End> {
        self.step_hook::<I, StepFailure, _>(
            instance,
            attempting,
            StepHook::OnStepFailure,
            <dyn BuiltStepPolicy<I::Workflow, I::Mode>>::on_step_failure,
            (attempting.attempt.clone(), cause),
            Some(failed),
        )
        .await
    }

    /// `on step retry`, once the step's own rule decided to attempt it again for `cause`.
    pub(super) async fn on_step_retry<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        attempting: &Attempting<'_, I::Workflow, I::Mode>,
        failed: &Failed,
        cause: RetryCause,
    ) -> Result<Option<Decided<FailWorkflow>>, End> {
        self.step_hook::<I, StepRetry, _>(
            instance,
            attempting,
            StepHook::OnStepRetry,
            <dyn BuiltStepPolicy<I::Workflow, I::Mode>>::on_step_retry,
            (attempting.attempt.clone(), cause),
            Some(failed),
        )
        .await
    }

    /// `on step abnormal termination`, after an attempt that ended in an abnormal termination.
    pub(super) async fn on_step_abnormal_termination<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        attempting: &Attempting<'_, I::Workflow, I::Mode>,
        failed: &Failed,
    ) -> Result<Option<Decided<FailWorkflow>>, End> {
        self.step_hook::<I, StepAbnormalTermination, _>(
            instance,
            attempting,
            StepHook::OnStepAbnormalTermination,
            <dyn BuiltStepPolicy<I::Workflow, I::Mode>>::on_step_abnormal_termination,
            attempting.attempt.clone(),
            Some(failed),
        )
        .await
    }

    /// `on workflow success`, once the journey succeeded, before `journey_succeeded`.
    pub(super) async fn on_workflow_success<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        descriptor: &WorkflowDescriptor<I::Workflow, I::Mode>,
        policies: &WorkflowPolicies<I::Workflow, I::Mode>,
    ) -> Result<(), End> {
        self.workflow_hook::<I, WorkflowSuccess>(
            instance,
            descriptor,
            policies,
            WorkflowHook::OnWorkflowSuccess,
            <dyn BuiltWorkflowPolicy<I::Workflow, I::Mode>>::on_workflow_success,
        )
        .await
    }

    /// `on workflow failure`, once the journey failed, before `journey_failed`.
    pub(super) async fn on_workflow_failure<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        descriptor: &WorkflowDescriptor<I::Workflow, I::Mode>,
        policies: &WorkflowPolicies<I::Workflow, I::Mode>,
    ) -> Result<(), End> {
        self.workflow_hook::<I, WorkflowFailure>(
            instance,
            descriptor,
            policies,
            WorkflowHook::OnWorkflowFailure,
            <dyn BuiltWorkflowPolicy<I::Workflow, I::Mode>>::on_workflow_failure,
        )
        .await
    }

    /// Calls a step hook, if a policy attached to the step defines it.
    async fn step_hook<I: WorkflowInstance, H: StepHookKind, L>(
        &mut self,
        instance: &mut I,
        attempting: &Attempting<'_, I::Workflow, I::Mode>,
        hook: StepHook,
        call: StepHookCall<I::Workflow, I::Mode, H, Option<L>>,
        context: H::Context,
        failed: Option<&Failed>,
    ) -> Result<Option<Decided<L>>, End>
    where
        Option<L>: Returned,
    {
        let Some((entry, policy)) = attempting.defining(hook) else {
            return Ok(None);
        };
        let name = entry.name();
        let called = Called {
            hook: HookSource::Step {
                policy: name,
                hook,
                step: attempting.attempt.clone(),
            },
            needs: entry.needs(hook),
            context,
            left: Some(Left {
                policy: name,
                hook,
                step: attempting.attempt.step,
                contributed: &attempting.contributed,
                failed,
            }),
        };
        match self.call_hook(instance, policy, call, called).await? {
            Some(lifecycle) => Ok(Some(Decided {
                policy: name,
                lifecycle,
            })),
            None => Ok(None),
        }
    }

    /// Calls a workflow hook, if a policy attached to the workflow defines it.
    async fn workflow_hook<I: WorkflowInstance, H: HookKind<Context = ()>>(
        &mut self,
        instance: &mut I,
        descriptor: &WorkflowDescriptor<I::Workflow, I::Mode>,
        policies: &WorkflowPolicies<I::Workflow, I::Mode>,
        hook: WorkflowHook,
        call: WorkflowHookCall<I::Workflow, I::Mode, H>,
    ) -> Result<(), End> {
        let defining = descriptor
            .policies()
            .iter()
            .zip(policies)
            .find(|(entry, _)| entry.defines(hook));
        let Some((entry, policy)) = defining else {
            return Ok(());
        };
        let called = Called {
            hook: HookSource::Workflow {
                policy: entry.name(),
                hook,
            },
            needs: entry.needs(hook),
            context: (),
            left: None,
        };
        self.call_hook(instance, policy.as_ref(), call, called)
            .await
    }

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
            let got = Requested::new(
                instance.workflow(),
                instance.journey_id(),
                data_bag,
                context,
                answers,
            );
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
    async fn answer<'s>(
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

    /// Calls the step's input adapter for one of its inputs, once what the adapter requests is
    /// resolved: the value it supplied, or `None` when it does not supply this input.
    pub(super) async fn adapt<W>(
        &mut self,
        sources: &Sources<'_, W>,
        adapter: &InputAdapterDescriptor<W>,
        attempt: &StepAttempt,
        input: &InputNeed,
    ) -> Result<Option<AnyValue>, End> {
        let name = adapter.name();
        let key = input.key();
        let requesting = Requesting::adapter(name, attempt);
        let data_bag = sources.data_bag;
        let mut answers = Answers::default();
        for request in adapter.needs().requests() {
            self.answer(&mut answers, &requesting, request, data_bag, None)
                .await?;
        }
        let got = Requested::new(
            sources.workflow,
            sources.journey_id,
            data_bag,
            (attempt.step, key),
            answers,
        );
        match adapter.adapt(sources.workflow, got) {
            Ok(Some(value)) => {
                self.emit(EventBody::InputAdapterSupplied {
                    step: attempt.clone(),
                    key: key.to_owned(),
                    adapter: name,
                })
                .await?;
                if input.accepts(&value) {
                    Ok(Some(value))
                } else {
                    Err(wrong_type(key, &requesting))
                }
            }
            Ok(None) => Ok(None),
            Err(error) => {
                self.emit(EventBody::InputAdapterFailed {
                    step: attempt.clone(),
                    key: key.to_owned(),
                    adapter: name,
                })
                .await?;
                Err(could_not_build(attempt.step, error))
            }
        }
    }
}

/// The abort for the reason or the error a step hook required from an attempt that has none.
fn missing_data(missing: MissingData, reported: event::MissingData) -> End {
    End::aborted(
        Abort::RequiredDataMissing(missing),
        JourneyAbort::RequiredDataMissing { missing: reported },
    )
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
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use rstest::rstest;

    use crate::engine::tests::{
        AMOUNT, CHARGE, Orders, Read, Recording, attempting, charge, crashed, data_of, declined,
        failing_on, instance, kinds, orders, scripted, succeed, timed_out, travel, travel_recorded,
        travel_workflow,
    };
    use crate::error::Error;
    use crate::event::{Event, EventBody, HookSource, JourneyAbort, RequestSource, Source};
    use crate::instance::WorkflowInstance;
    use crate::journey::{Abort, JourneyStatus, MissingData, Requester};
    use crate::policy::{
        FailWorkflow, HookNeeds, OnStepAbnormalTermination, OnStepFailure, OnStepRetry,
        OnStepSuccess, OnSuccess, OnWorkflowFailure, OnWorkflowSuccess, PolicyName, Provides,
        Requested, StepAbnormalTermination, StepFailure, StepHook, StepPolicyDescriptor, StepRetry,
        StepSuccess, WorkflowFailure, WorkflowHook, WorkflowPolicyDescriptor, WorkflowSuccess,
    };
    use crate::step::{Input, OptionalInput, Outcome, StepAttempt, StepDescriptor};
    use crate::workflow::WorkflowBuilder;

    const DECLINE: Input<String> = Input::new("decline");
    const DRAFT: OptionalInput<String> = OptionalInput::new("draft");
    const COUPON: OptionalInput<String> = OptionalInput::new("coupon");

    /// What hooks saw, one entry per call.
    type Seen = Arc<Mutex<Vec<String>>>;

    fn seen(seen: &Seen) -> Vec<String> {
        seen.lock().unwrap().clone()
    }

    /// `on step failure`, which requests data from the step and from the workflow, each
    /// required and optional, the reason and the optional error, and records what it got.
    struct Alarm {
        seen: Seen,
    }

    impl OnStepFailure<Orders> for Alarm {
        fn needs() -> HookNeeds<StepFailure> {
            HookNeeds::new()
                .from_step(&DECLINE)
                .optional_from_step(&DRAFT)
                .from_workflow(&AMOUNT)
                .optional_from_workflow(&COUPON)
                .reason()
                .optional_error()
        }

        fn on_step_failure(
            &self,
            mut got: Requested<'_, Orders, StepFailure>,
        ) -> Result<Option<FailWorkflow>, Error> {
            let saw = format!(
                "{} {:?} {} {:?} {} {} {} {} {} {}",
                got.from_step(&DECLINE)?,
                got.optional_from_step(&DRAFT)?,
                got.from_workflow(&AMOUNT)?,
                got.optional_from_workflow(&COUPON)?,
                got.reason()?.code(),
                got.optional_error()?.is_some(),
                got.cause(),
                got.step_name(),
                got.attempt(),
                got.journey_id(),
            );
            self.seen.lock().unwrap().push(saw);
            Ok(None)
        }
    }

    fn charge_declined_with_a_contribution() -> StepDescriptor<Orders> {
        scripted(|contributor, _| {
            contributor.contribute("decline", "insufficient funds".to_string());
            declined()
        })
    }

    #[test]
    fn a_step_hook_receives_what_it_requests_from_the_step_and_the_workflow_before_it_runs() {
        let saw = Seen::default();
        let alarm = Arc::clone(&saw);
        let policy = StepPolicyDescriptor::new("alarm", move || Alarm {
            seen: Arc::clone(&alarm),
        })
        .on_step_failure();
        let workflow = orders().step(charge_declined_with_a_contribution().policy(policy));
        let instance = instance(workflow).data("amount", 42_i64).create().unwrap();
        let journey_id = instance.journey_id().clone();

        let (status, events) = travel(instance);

        assert_eq!(
            seen(&saw),
            [format!(
                "insufficient funds None 42 None declined false failure charge 1 {journey_id}"
            )]
        );
        assert!(matches!(status, JourneyStatus::Failed { .. }));
        assert!(status.data().unwrap().get("decline").is_none());
        let hook = HookSource::Step {
            policy: PolicyName::from("alarm"),
            hook: StepHook::OnStepFailure,
            step: StepAttempt::first(CHARGE),
        };
        let absent: Vec<(&str, &RequestSource)> = events.iter().filter_map(absent_key).collect();
        assert_eq!(
            absent,
            [
                ("draft", &RequestSource::Hook(hook.clone())),
                ("coupon", &RequestSource::Hook(hook))
            ]
        );
        assert_eq!(
            kinds(&events)[3..],
            [
                "step_given_up",
                "optional_input_absent",
                "optional_input_absent",
                "hook_called",
                "journey_failed"
            ]
        );
    }

    fn absent_key(event: &Event) -> Option<(&str, &RequestSource)> {
        match &event.body {
            EventBody::OptionalInputAbsent { key, requester } => Some((key, requester)),
            _ => None,
        }
    }

    /// `on step failure`, which ignores what it requested.
    struct Silent;

    impl OnStepFailure<Orders> for Silent {
        fn on_step_failure(
            &self,
            _: Requested<'_, Orders, StepFailure>,
        ) -> Result<Option<FailWorkflow>, Error> {
            Ok(None)
        }
    }

    /// How the charge step ends.
    type Ends = fn() -> Result<Outcome, Error>;

    /// Runs a journey whose charge step ends as `ends` says, with a policy needing `needs`, and
    /// returns the result's status and the events.
    fn travel_needing(needs: HookNeeds<StepFailure>, ends: Ends) -> (JourneyStatus, Vec<Event>) {
        let policy = StepPolicyDescriptor::new("alarm", || Silent).on_step_failure_needing(needs);
        travel_workflow(orders().step(StepDescriptor::new(CHARGE, ends).policy(policy)))
    }

    /// The charge step, which succeeds.
    fn charge_succeeding() -> StepDescriptor<Orders> {
        StepDescriptor::new(CHARGE, succeed)
    }

    /// `on step failure`, which records the data from the step it was attached to read, which its
    /// own type does not declare.
    struct Echo {
        key: &'static str,
        seen: Seen,
    }

    impl OnStepFailure<Orders> for Echo {
        fn on_step_failure(
            &self,
            mut got: Requested<'_, Orders, StepFailure>,
        ) -> Result<Option<FailWorkflow>, Error> {
            let value = got.from_step(&Input::<String>::new(self.key))?;
            self.seen.lock().unwrap().push(value);
            Ok(None)
        }
    }

    #[test]
    fn a_hook_needs_what_its_attachment_declares() {
        let saw = Seen::default();
        let echo = Arc::clone(&saw);
        let policy = StepPolicyDescriptor::new("echo", move || Echo {
            key: "decline",
            seen: Arc::clone(&echo),
        })
        .on_step_failure_needing(HookNeeds::new().from_step(&DECLINE));

        travel_workflow(orders().step(charge_declined_with_a_contribution().policy(policy)));

        assert_eq!(seen(&saw), ["insufficient funds"]);
    }

    /// `on step abnormal termination`, which records the draft the attempt contributed.
    struct Drafts {
        seen: Seen,
    }

    impl OnStepAbnormalTermination<Orders> for Drafts {
        fn needs() -> HookNeeds<StepAbnormalTermination> {
            HookNeeds::new().optional_from_step(&DRAFT)
        }

        fn on_step_abnormal_termination(
            &self,
            mut got: Requested<'_, Orders, StepAbnormalTermination>,
        ) -> Result<Option<FailWorkflow>, Error> {
            let draft = got.optional_from_step(&DRAFT)?;
            self.seen.lock().unwrap().push(format!("{draft:?}"));
            Ok(None)
        }
    }

    #[test]
    fn an_abnormal_termination_carries_no_contributions_to_its_hooks() {
        let saw = Seen::default();
        let drafts = Arc::clone(&saw);
        let policy = StepPolicyDescriptor::new("drafts", move || Drafts {
            seen: Arc::clone(&drafts),
        })
        .on_step_abnormal_termination();
        let step = scripted(|contributor, _| {
            contributor.contribute("draft", "D-1".to_string());
            crashed()
        });

        let (_, events) = travel_workflow(orders().step(step.policy(policy)));

        assert_eq!(seen(&saw), ["None"]);
        assert!(kinds(&events).contains(&"optional_input_absent"));
    }

    fn alarm_requester() -> Requester {
        Requester::StepHook {
            policy: PolicyName::from("alarm"),
            hook: StepHook::OnStepFailure,
        }
    }

    #[rstest]
    #[case::data_from_the_step(
        HookNeeds::new().from_step(&DECLINE),
        declined,
        MissingData::Key { key: "decline".to_string(), requester: alarm_requester() }
    )]
    #[case::data_from_the_workflow(
        HookNeeds::new().from_workflow(&AMOUNT),
        declined,
        MissingData::Key { key: "amount".to_string(), requester: alarm_requester() }
    )]
    #[case::the_reason_of_an_abnormal_termination(
        HookNeeds::new().reason(),
        crashed,
        MissingData::Reason {
            policy: PolicyName::from("alarm"),
            hook: StepHook::OnStepFailure,
        }
    )]
    #[case::the_error_of_a_failure(
        HookNeeds::new().error(),
        declined,
        MissingData::Error {
            policy: PolicyName::from("alarm"),
            hook: StepHook::OnStepFailure,
        }
    )]
    fn a_required_request_without_a_value_aborts_the_journey_before_the_hook_runs(
        #[case] needs: HookNeeds<StepFailure>,
        #[case] ends: Ends,
        #[case] expected: MissingData,
    ) {
        let (status, events) = travel_needing(needs, ends);

        let JourneyStatus::Aborted(Abort::RequiredDataMissing(missing)) = status else {
            panic!("the journey was not aborted for missing data: {status:?}");
        };
        assert_eq!(missing, expected);
        assert!(!kinds(&events).contains(&"hook_called"));
        assert!(matches!(
            events.last().map(|event| &event.body),
            Some(EventBody::JourneyAborted { abort }) if abort.step() == Some(CHARGE)
        ));
    }

    #[rstest]
    #[case::the_error_before_the_draft(
        HookNeeds::new().error().optional_from_step(&DRAFT),
        &["journey_started", "attempt_started", "step_failed", "step_given_up", "journey_aborted"]
    )]
    #[case::the_draft_before_the_error(
        HookNeeds::new().optional_from_step(&DRAFT).error(),
        &[
            "journey_started",
            "attempt_started",
            "step_failed",
            "step_given_up",
            "optional_input_absent",
            "journey_aborted"
        ]
    )]
    fn a_hooks_requests_are_resolved_in_the_order_declared(
        #[case] needs: HookNeeds<StepFailure>,
        #[case] expected: &[&str],
    ) {
        let (status, events) = travel_needing(needs, declined);

        assert!(matches!(
            status,
            JourneyStatus::Aborted(Abort::RequiredDataMissing(MissingData::Error { .. }))
        ));
        assert_eq!(kinds(&events), expected);
    }

    #[test]
    fn a_reason_declared_again_replaces_the_earlier_declaration() {
        let (status, _) = travel_needing(HookNeeds::new().reason().optional_reason(), crashed);

        assert!(matches!(status, JourneyStatus::Failed { .. }));
    }

    /// `on workflow success`, which requests the amount from the workflow.
    struct Totals;

    impl OnWorkflowSuccess<Orders> for Totals {
        fn needs() -> HookNeeds<WorkflowSuccess> {
            HookNeeds::new().from_workflow(&AMOUNT)
        }

        fn on_workflow_success(
            &self,
            _: Requested<'_, Orders, WorkflowSuccess>,
        ) -> Result<(), Error> {
            Ok(())
        }
    }

    #[test]
    fn a_value_of_another_type_for_a_hook_aborts_the_journey_naming_the_hook() {
        let workflow = orders()
            .step(charge_succeeding())
            .policy(WorkflowPolicyDescriptor::new("totals", || Totals).on_workflow_success());

        let (status, events) = travel(
            instance(workflow)
                .data("amount", "forty".to_string())
                .create()
                .unwrap(),
        );

        let JourneyStatus::Aborted(Abort::WrongType { key, requester }) = status else {
            panic!("the journey was not aborted for a wrong type: {status:?}");
        };
        assert_eq!(key, "amount");
        assert_eq!(
            requester,
            Requester::WorkflowHook {
                policy: PolicyName::from("totals"),
                hook: WorkflowHook::OnWorkflowSuccess,
            }
        );
        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                "step_succeeded",
                "journey_aborted"
            ]
        );
    }

    /// `on step success`, which reports and contributes a new attempt number.
    struct Renumber;

    impl OnStepSuccess<Orders> for Renumber {
        fn needs() -> HookNeeds<StepSuccess> {
            HookNeeds::new().contributor().reporter()
        }

        fn on_step_success(
            &self,
            mut got: Requested<'_, Orders, StepSuccess>,
        ) -> Result<Option<OnSuccess>, Error> {
            got.reporter()?.info("renumbering")?;
            got.contributor()?.contribute("attempt", 10_i64);
            Ok(None)
        }
    }

    #[test]
    fn a_hooks_contributions_are_committed_once_it_returns_with_the_hook_as_their_source() {
        let step = attempting(&[])
            .policy(StepPolicyDescriptor::new("renumber", || Renumber).on_step_success());

        let (status, events) = travel_workflow(orders().step(step));

        assert_eq!(data_of(&status), [("attempt".to_string(), 10)]);
        let hook = HookSource::Step {
            policy: PolicyName::from("renumber"),
            hook: StepHook::OnStepSuccess,
            step: StepAttempt::first(CHARGE),
        };
        let sources: Vec<(&str, &Source)> = events.iter().filter_map(committed).collect();
        assert_eq!(
            sources,
            [
                (
                    "contribution_committed",
                    &Source::Step(StepAttempt::first(CHARGE))
                ),
                ("contribution_committed", &Source::Hook(hook.clone())),
                ("data_overwritten", &Source::Hook(hook.clone())),
            ]
        );
        assert_eq!(
            kinds(&events)[4..],
            [
                "journey_info",
                "hook_called",
                "contribution_committed",
                "data_overwritten",
                "journey_succeeded"
            ]
        );
        assert!(matches!(
            events.get(4).map(|event| &event.body),
            Some(EventBody::JourneyInfo { hook: stamped, message, .. })
                if *stamped == hook && message == "renumbering"
        ));
    }

    fn committed(event: &Event) -> Option<(&'static str, &Source)> {
        match &event.body {
            EventBody::ContributionCommitted { source, .. }
            | EventBody::DataOverwritten { source, .. } => Some((event.kind(), source)),
            _ => None,
        }
    }

    #[test]
    fn a_reporter_failing_on_a_hooks_event_aborts_the_journey_whatever_the_hook_does_next() {
        let step = attempting(&[])
            .policy(StepPolicyDescriptor::new("renumber", || Renumber).on_step_success());
        let recording = Recording::default();
        let events = Arc::clone(&recording.events);
        let instance = instance(orders().step(step)).create().unwrap();

        let status = travel_recorded(instance, recording, Some(failing_on("journey_info")));

        assert!(matches!(
            status,
            JourneyStatus::Aborted(Abort::ReporterFailed(_))
        ));
        let events = events.lock().unwrap().clone();
        assert_eq!(kinds(&events)[4..], ["journey_info", "journey_aborted"]);
    }

    /// `on step success`, which fails.
    struct Ledger;

    impl OnStepSuccess<Orders> for Ledger {
        fn on_step_success(
            &self,
            _: Requested<'_, Orders, StepSuccess>,
        ) -> Result<Option<OnSuccess>, Error> {
            Err(Error::msg("the ledger is closed"))
        }
    }

    #[test]
    fn a_hook_that_fails_aborts_the_journey_with_its_error() {
        let step = charge_succeeding()
            .policy(StepPolicyDescriptor::new("ledger", || Ledger).on_step_success());

        let (status, events) = travel_workflow(orders().step(step));

        let JourneyStatus::Aborted(Abort::HookFailed(error)) = status else {
            panic!("the journey was not aborted by the hook: {status:?}");
        };
        assert_eq!(error.to_string(), "the ledger is closed");
        assert!(matches!(
            events.last().map(|event| &event.body),
            Some(EventBody::JourneyAborted { abort }) if *abort == JourneyAbort::HookFailed {
                step: Some(CHARGE),
                error: "the ledger is closed".to_string(),
            }
        ));
        assert!(!kinds(&events).contains(&"hook_called"));
    }

    /// `on step retry`, which counts how many instances of it were built.
    struct Counted;

    impl OnStepRetry<Orders> for Counted {
        fn on_step_retry(
            &self,
            _: Requested<'_, Orders, StepRetry>,
        ) -> Result<Option<FailWorkflow>, Error> {
            Ok(None)
        }
    }

    #[test]
    fn step_policies_are_built_for_every_attempt() {
        let built = Arc::new(AtomicUsize::new(0));
        let counting = Arc::clone(&built);
        let policy = StepPolicyDescriptor::new("counted", move || {
            counting.fetch_add(1, Ordering::SeqCst);
            Counted
        })
        .on_step_retry();
        let step = attempting(&[timed_out]).retry_budget(1).policy(policy);

        let (status, _) = travel_workflow(orders().step(step));

        assert!(matches!(status, JourneyStatus::Succeeded { .. }));
        assert_eq!(built.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn a_step_policy_that_cannot_be_built_aborts_the_journey_before_the_steps_inputs() {
        let step = charge(&Read::default()).policy(broken());

        let (status, events) = travel_workflow(orders().step(step));

        let JourneyStatus::Aborted(Abort::PolicyCouldNotBeBuilt { policy, error }) = status else {
            panic!("the journey was not aborted by the policy: {status:?}");
        };
        assert_eq!(policy, PolicyName::from("counted"));
        assert_eq!(error.to_string(), "no counter");
        assert_eq!(
            kinds(&events),
            ["journey_started", "attempt_started", "journey_aborted"]
        );
    }

    /// The workflow hooks, which record which of them was called.
    struct Close {
        seen: Seen,
    }

    impl OnWorkflowSuccess<Orders> for Close {
        fn on_workflow_success(
            &self,
            _: Requested<'_, Orders, WorkflowSuccess>,
        ) -> Result<(), Error> {
            self.seen.lock().unwrap().push("success".to_string());
            Ok(())
        }
    }

    impl OnWorkflowFailure<Orders> for Close {
        fn on_workflow_failure(
            &self,
            _: Requested<'_, Orders, WorkflowFailure>,
        ) -> Result<(), Error> {
            self.seen.lock().unwrap().push("failure".to_string());
            Ok(())
        }
    }

    fn closing(seen: &Seen) -> WorkflowBuilder<Orders> {
        let close = Arc::clone(seen);
        orders().policy(
            WorkflowPolicyDescriptor::new("close", move || Close {
                seen: Arc::clone(&close),
            })
            .on_workflow_success()
            .on_workflow_failure(),
        )
    }

    #[rstest]
    #[case::a_success(succeed, &["success"], "journey_succeeded")]
    #[case::a_failure(declined, &["failure"], "journey_failed")]
    fn the_workflow_hook_for_how_the_journey_ended_runs_before_it_is_reported(
        #[case] ends: fn() -> Result<Outcome, Error>,
        #[case] called: &[&str],
        #[case] last: &str,
    ) {
        let saw = Seen::default();
        let workflow = closing(&saw).step(StepDescriptor::new(CHARGE, ends));

        let (_, events) = travel_workflow(workflow);

        assert_eq!(seen(&saw), called);
        assert_eq!(kinds(&events)[events.len() - 2..], ["hook_called", last]);
    }

    /// What the workflow knows of its customers' balances, a role it provides to its policies.
    trait Balances {
        fn balance(&self) -> i64;
    }

    impl Balances for Orders {
        fn balance(&self) -> i64 {
            42
        }
    }

    impl Provides<dyn Balances> for Orders {
        fn role(&self) -> &(dyn Balances + 'static) {
            self
        }
    }

    /// `on workflow success`, which contributes the balance the workflow's role gives.
    struct Settle;

    impl OnWorkflowSuccess<Orders> for Settle {
        fn needs() -> HookNeeds<WorkflowSuccess> {
            HookNeeds::new().contributor()
        }

        fn on_workflow_success(
            &self,
            mut got: Requested<'_, Orders, WorkflowSuccess>,
        ) -> Result<(), Error> {
            let balance = got.role::<dyn Balances>().balance();
            got.contributor()?.contribute("balance", balance);
            Ok(())
        }
    }

    #[test]
    fn a_workflow_hooks_contributions_are_committed_before_the_journey_is_reported() {
        let workflow = orders()
            .policy(WorkflowPolicyDescriptor::new("settle", || Settle).on_workflow_success())
            .step(charge_succeeding());

        let (status, events) = travel_workflow(workflow);

        assert_eq!(data_of(&status), [("balance".to_string(), 42)]);
        assert_eq!(
            kinds(&events)[events.len() - 3..],
            ["hook_called", "contribution_committed", "journey_succeeded"]
        );
    }

    /// A step policy that cannot be built.
    fn broken() -> StepPolicyDescriptor<Counted, Orders> {
        StepPolicyDescriptor::fallible("counted", || Err(Error::msg("no counter"))).on_step_retry()
    }

    #[test]
    fn no_workflow_hook_runs_when_the_journey_is_aborted() {
        let saw = Seen::default();
        let workflow = closing(&saw).step(charge_succeeding().policy(broken()));

        let (status, _) = travel_workflow(workflow);

        assert!(matches!(status, JourneyStatus::Aborted(_)));
        assert!(seen(&saw).is_empty());
    }
}
