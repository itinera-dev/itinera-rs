//! The journeys the engine's tests run: the orders workflow, its steps and policies, and a
//! reporter that records what each journey emitted.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{Failures, Inline, finish, run};
use crate::error::Error;
use crate::event::{Event, EventBody, JourneyFailure};
use crate::executor::build_policies;
use crate::instance::{InstanceBuilder, WorkflowInstance};
use crate::journey::{Contributor, Failure, JourneyStatus};
#[cfg(feature = "async")]
use crate::mode::Asynchronous;
use crate::policy::{
    FailWorkflow, HookNeeds, Lifecycle, OnStepAbnormalTermination, OnStepFailure, OnStepRetry,
    OnStepSuccess, OnSuccess, PolicyName, Requested, StepAbnormalTermination, StepFailure,
    StepHook, StepPolicyDescriptor, StepRetry, StepSuccess,
};
use crate::report::{DefaultDispatcher, Dispatcher, Reporter};
#[cfg(feature = "async")]
use crate::step::{AsyncStep, AsyncStepFactory};
use crate::step::{
    Input, OptionalInput, Outcome, Reason, Resolved, Step, StepDescriptor, StepFactory, StepName,
    StepNeeds, StepReporter,
};
use crate::value::AnyValue;
use crate::workflow::fixtures::{Orders, orders};
use crate::workflow::{WorkflowBuilder, WorkflowDescriptor};

/// A reporter that records every event it receives, and fails on one kind if told to.
#[derive(Default)]
pub(super) struct Recording {
    pub(super) events: Arc<Mutex<Vec<Event>>>,
    fails_on: Option<&'static str>,
}

impl Reporter for Recording {
    fn report(&mut self, event: &Event) -> Result<(), Error> {
        self.events.lock().unwrap().push(event.clone());
        if self.fails_on == Some(event.kind()) {
            return Err(Error::msg(format!("failed on {}", event.kind())));
        }
        Ok(())
    }
}

pub(crate) const CHARGE: StepName = StepName::new("charge");
pub(super) const SHIP: StepName = StepName::new("ship");
pub(super) const AMOUNT: Input<i64> = Input::new("amount");
const DISCOUNT: OptionalInput<i64> = OptionalInput::new("discount");

/// What the charge step read, one entry per attempt.
pub(super) type Read = Arc<Mutex<Vec<(i64, Option<i64>)>>>;

/// The factory of a step that needs an amount, may have a discount, and records what it read.
pub(super) struct Charge {
    pub(super) read: Read,
}

pub(super) struct Charging<'a> {
    amount: i64,
    discount: Option<i64>,
    read: &'a Read,
}

impl Charge {
    fn inputs() -> StepNeeds {
        StepNeeds::new().input(&AMOUNT).optional_input(&DISCOUNT)
    }

    fn charging<'a, M>(&'a self, got: &mut Resolved<'a, M>) -> Result<Charging<'a>, Error> {
        Ok(Charging {
            amount: got.input(&AMOUNT)?,
            discount: got.optional_input(&DISCOUNT)?,
            read: &self.read,
        })
    }
}

impl StepFactory for Charge {
    type Step<'a> = Charging<'a>;

    fn needs(&self) -> StepNeeds {
        Self::inputs()
    }

    fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Charging<'a>, Error> {
        self.charging(got)
    }
}

impl Charging<'_> {
    fn record(self) -> Result<Outcome, Error> {
        self.read.lock().unwrap().push((self.amount, self.discount));
        Ok(Outcome::success())
    }
}

impl Step for Charging<'_> {
    fn run(self) -> Result<Outcome, Error> {
        self.record()
    }
}

/// The factory of a step that contributes and emits as its script says, and reports the
/// outcome the script returns.
struct Scripted<S> {
    script: S,
}

struct ScriptedStep<'a, S> {
    script: &'a S,
    contributor: Contributor<'a>,
    reporter: StepReporter<'a>,
}

pub(crate) fn scripted<S>(script: S) -> StepDescriptor<Orders>
where
    S: Fn(&mut Contributor<'_>, &mut StepReporter<'_>) -> Result<Outcome, Error>
        + Send
        + Sync
        + 'static,
{
    StepDescriptor::new(CHARGE, Scripted { script })
}

impl<S> StepFactory for Scripted<S>
where
    S: Fn(&mut Contributor<'_>, &mut StepReporter<'_>) -> Result<Outcome, Error>
        + Send
        + Sync
        + 'static,
{
    type Step<'a> = ScriptedStep<'a, S>;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new().contributor().reporter()
    }

    fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<ScriptedStep<'a, S>, Error> {
        Ok(ScriptedStep {
            script: &self.script,
            contributor: got.contributor()?,
            reporter: got.reporter()?,
        })
    }
}

impl<S> Step for ScriptedStep<'_, S>
where
    S: Fn(&mut Contributor<'_>, &mut StepReporter<'_>) -> Result<Outcome, Error>,
{
    fn run(mut self) -> Result<Outcome, Error> {
        (self.script)(&mut self.contributor, &mut self.reporter)
    }
}

pub(super) fn failing_on(kind: &'static str) -> Recording {
    Recording {
        fails_on: Some(kind),
        ..Recording::default()
    }
}

pub(super) fn noon() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_700_000_000)
}

/// Runs the journey to its end, and returns its status and the events it emitted.
pub(super) fn travel<I: WorkflowInstance>(instance: I) -> (JourneyStatus, Vec<Event>) {
    travel_before(instance, None)
}

/// Runs the journey to its end with a recording reporter, before the one given if any, and
/// returns its status and the events the recording reporter received.
pub(super) fn travel_before<I: WorkflowInstance>(
    instance: I,
    after: Option<Recording>,
) -> (JourneyStatus, Vec<Event>) {
    let recording = Recording::default();
    let events = Arc::clone(&recording.events);
    let status = travel_recorded(instance, recording, after);
    let events = events.lock().unwrap().clone();
    (status, events)
}

/// Runs a journey of the instance, delivering its events to the recording, then to `after`
/// if given.
pub(super) fn travel_recorded<I: WorkflowInstance>(
    instance: I,
    recording: Recording,
    after: Option<Recording>,
) -> JourneyStatus {
    let policies = build_policies(&instance).unwrap();
    let failures = Failures::default();
    let mut dispatcher = DefaultDispatcher::new();
    for reporter in [recording].into_iter().chain(after) {
        dispatcher.add(failures.guard(Box::new(reporter))).unwrap();
    }
    finish(run(
        instance,
        policies,
        Inline::from(dispatcher),
        failures,
        noon,
    ))
    .status
}

/// The charge step, which needs an amount, may have a discount, and records what it read
/// in `read`.
pub(super) fn charge(read: &Read) -> StepDescriptor<Orders> {
    let charge = Charge {
        read: Arc::clone(read),
    };
    StepDescriptor::new(CHARGE, charge)
}

/// The orders workflow with the charge step.
pub(super) fn charged(read: &Read) -> WorkflowBuilder<Orders> {
    orders().step(charge(read))
}

/// Runs a journey of the workflow, whose initial data holds an amount of 42.
pub(super) fn travel_with_amount(builder: WorkflowBuilder<Orders>) -> (JourneyStatus, Vec<Event>) {
    travel(instance(builder).data("amount", 42_i64).create().unwrap())
}

/// Runs a journey of the workflow, with no initial data.
pub(crate) fn travel_workflow(builder: WorkflowBuilder<Orders>) -> (JourneyStatus, Vec<Event>) {
    travel(instance(builder).create().unwrap())
}

pub(super) fn instance(builder: WorkflowBuilder<Orders>) -> InstanceBuilder<Orders> {
    builder.build().unwrap().instance(Orders)
}

pub(super) fn succeed() -> Result<Outcome, Error> {
    Ok(Outcome::success())
}

pub(crate) fn kinds(events: &[Event]) -> Vec<&'static str> {
    events.iter().map(Event::kind).collect()
}

pub(super) fn reads(read: &Read) -> Vec<(i64, Option<i64>)> {
    read.lock().unwrap().clone()
}

pub(super) type Data = fn(InstanceBuilder<Orders>) -> InstanceBuilder<Orders>;

pub(super) fn amount_as_i32(journey: InstanceBuilder<Orders>) -> InstanceBuilder<Orders> {
    journey.data("amount", 42_i32)
}

pub(super) fn declined() -> Result<Outcome, Error> {
    Ok(Outcome::failure(Reason::new("declined")))
}

pub(super) fn timed_out() -> Result<Outcome, Error> {
    Ok(Outcome::retriable_failure(Reason::new("timeout")))
}

pub(crate) fn crashed() -> Result<Outcome, Error> {
    Err(Error::msg("the gateway crashed"))
}

/// A step that ends each attempt as the next of `ends` says, and succeeds once none is left.
/// Each attempt contributes its own number as "attempt".
pub(super) fn attempting(
    ends: &'static [fn() -> Result<Outcome, Error>],
) -> StepDescriptor<Orders> {
    let made = AtomicUsize::new(0);
    scripted(move |contributor, _| {
        let attempt = made.fetch_add(1, Ordering::SeqCst);
        contributor.contribute("attempt", i64::try_from(attempt + 1).unwrap());
        ends.get(attempt).map_or_else(succeed, call)
    })
}

fn call(end: &fn() -> Result<Outcome, Error>) -> Result<Outcome, Error> {
    end()
}

/// A step policy, "audit", whose hook `hook` returns `lifecycle` and whose other hooks
/// return nothing. It records each call of its hooks with how many events had been delivered
/// when it was made.
#[derive(Clone)]
struct Answering {
    hook: StepHook,
    lifecycle: Lifecycle,
    events: Arc<Mutex<Vec<Event>>>,
    calls: Arc<Mutex<Vec<(StepHook, usize)>>>,
}

pub(super) fn audit() -> PolicyName {
    PolicyName::from("audit")
}

impl Answering {
    fn called(&self, hook: StepHook) -> Option<Lifecycle> {
        let delivered = self.events.lock().unwrap().len();
        self.calls.lock().unwrap().push((hook, delivered));
        (hook == self.hook).then(|| self.lifecycle.clone())
    }
}

impl OnStepSuccess<Orders> for Answering {
    fn on_step_success(
        &self,
        _: Requested<'_, Orders, StepSuccess>,
    ) -> Result<Option<OnSuccess>, Error> {
        Ok(self.called(StepHook::OnStepSuccess).map(on_success))
    }
}

impl OnStepFailure<Orders> for Answering {
    fn on_step_failure(
        &self,
        _: Requested<'_, Orders, StepFailure>,
    ) -> Result<Option<FailWorkflow>, Error> {
        Ok(self.called(StepHook::OnStepFailure).and_then(failing))
    }
}

impl OnStepRetry<Orders> for Answering {
    fn on_step_retry(
        &self,
        _: Requested<'_, Orders, StepRetry>,
    ) -> Result<Option<FailWorkflow>, Error> {
        Ok(self.called(StepHook::OnStepRetry).and_then(failing))
    }
}

impl OnStepAbnormalTermination<Orders> for Answering {
    fn on_step_abnormal_termination(
        &self,
        _: Requested<'_, Orders, StepAbnormalTermination>,
    ) -> Result<Option<FailWorkflow>, Error> {
        Ok(self
            .called(StepHook::OnStepAbnormalTermination)
            .and_then(failing))
    }
}

fn on_success(lifecycle: Lifecycle) -> OnSuccess {
    match lifecycle {
        Lifecycle::FinishWorkflow => OnSuccess::FinishWorkflow,
        Lifecycle::FailWorkflow(reason) => OnSuccess::FailWorkflow(reason),
    }
}

fn failing(lifecycle: Lifecycle) -> Option<FailWorkflow> {
    match lifecycle {
        Lifecycle::FailWorkflow(reason) => Some(reason.into()),
        Lifecycle::FinishWorkflow => None,
    }
}

/// What a journey whose hooks were scripted gave: its status, its events, and the hooks
/// called, each with how many events had been delivered before it.
pub(super) struct Hooked {
    pub(super) status: JourneyStatus,
    pub(super) events: Vec<Event>,
    pub(super) calls: Vec<(StepHook, usize)>,
}

/// Runs a journey of a workflow whose first step has the policy "audit", whose hook `hook`
/// returns `lifecycle`, and whose second step, if any, has no policy.
pub(super) fn travel_hooked(
    first: StepDescriptor<Orders>,
    second: Option<StepDescriptor<Orders>>,
    hook: StepHook,
    lifecycle: Lifecycle,
) -> Hooked {
    let recording = Recording::default();
    let events = Arc::clone(&recording.events);
    let calls = Arc::default();
    let answering = Answering {
        hook,
        lifecycle,
        events: Arc::clone(&events),
        calls: Arc::clone(&calls),
    };
    let policy = StepPolicyDescriptor::new("audit", move || answering.clone())
        .on_step_success()
        .on_step_failure()
        .on_step_retry()
        .on_step_abnormal_termination();
    let builder = second
        .into_iter()
        .fold(orders().step(first.policy(policy)), WorkflowBuilder::step);
    let instance = instance(builder).create().unwrap();
    let status = travel_recorded(instance, recording, None);
    let events = events.lock().unwrap().clone();
    let calls = calls.lock().unwrap().clone();
    Hooked {
        status,
        events,
        calls,
    }
}

pub(super) fn fail_workflow() -> Lifecycle {
    Lifecycle::FailWorkflow(Reason::new("fraud"))
}

fn decided_by(event: &Event) -> Option<(PolicyName, StepHook)> {
    match &event.body {
        EventBody::JourneyFailed {
            failure: JourneyFailure::FailWorkflow { decided_by, .. },
            ..
        } => Some((decided_by.policy, decided_by.hook)),
        _ => None,
    }
}

pub(super) fn failed_by_the_hook(hooked: &Hooked, hook: StepHook) {
    let JourneyStatus::Failed { failure, .. } = &hooked.status else {
        panic!("the journey did not fail: {:?}", hooked.status);
    };
    assert!(matches!(failure, Failure::FailWorkflow(reason) if reason.code() == "fraud"));
    assert_eq!(
        hooked.events.last().and_then(decided_by),
        Some((audit(), hook))
    );
}

pub(super) fn data_of(status: &JourneyStatus) -> Vec<(String, i64)> {
    status
        .data()
        .into_iter()
        .flatten()
        .map(key_and_number)
        .collect()
}

fn key_and_number((key, value): (&String, &AnyValue)) -> (String, i64) {
    (key.clone(), value.downcast_ref::<i64>().copied().unwrap())
}

pub(super) fn skips() -> Result<Outcome, Error> {
    Ok(Outcome::skipped())
}

impl Step for StepReporter<'_> {
    fn run(self) -> Result<Outcome, Error> {
        Ok(Outcome::success())
    }
}

#[cfg(feature = "async")]
pub(super) fn async_orders() -> WorkflowBuilder<Orders, Asynchronous> {
    WorkflowDescriptor::async_builder("orders")
}

#[cfg(feature = "async")]
impl AsyncStepFactory for Charge {
    type Step<'a> = Charging<'a>;

    fn needs(&self) -> StepNeeds {
        Self::inputs()
    }

    fn build<'a>(&'a self, got: &mut Resolved<'a, Asynchronous>) -> Result<Charging<'a>, Error> {
        self.charging(got)
    }
}

#[cfg(feature = "async")]
impl AsyncStep for Charging<'_> {
    async fn run(self) -> Result<Outcome, Error> {
        self.record()
    }
}

pub(super) const DECLINE: Input<String> = Input::new("decline");
pub(super) const DRAFT: OptionalInput<String> = OptionalInput::new("draft");

/// What hooks saw, one entry per call.
pub(super) type Seen = Arc<Mutex<Vec<String>>>;

pub(super) fn seen(seen: &Seen) -> Vec<String> {
    seen.lock().unwrap().clone()
}

pub(super) fn charge_declined_with_a_contribution() -> StepDescriptor<Orders> {
    scripted(|contributor, _| {
        contributor.contribute("decline", "insufficient funds".to_string());
        declined()
    })
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
pub(super) type Ends = fn() -> Result<Outcome, Error>;

/// Runs a journey whose charge step ends as `ends` says, with a policy needing `needs`, and
/// returns the result's status and the events.
pub(crate) fn travel_needing(
    needs: HookNeeds<StepFailure>,
    ends: Ends,
) -> (JourneyStatus, Vec<Event>) {
    let policy = StepPolicyDescriptor::new("alarm", || Silent).on_step_failure_needing(needs);
    travel_workflow(orders().step(StepDescriptor::new(CHARGE, ends).policy(policy)))
}

/// The charge step, which succeeds.
pub(super) fn charge_succeeding() -> StepDescriptor<Orders> {
    StepDescriptor::new(CHARGE, succeed)
}

/// `on step retry`, which counts how many instances of it were built.
pub(super) struct Counted;

impl OnStepRetry<Orders> for Counted {
    fn on_step_retry(
        &self,
        _: Requested<'_, Orders, StepRetry>,
    ) -> Result<Option<FailWorkflow>, Error> {
        Ok(None)
    }
}

/// A step policy that cannot be built.
pub(super) fn broken() -> StepPolicyDescriptor<Counted, Orders> {
    StepPolicyDescriptor::fallible("counted", || Err(Error::msg("no counter"))).on_step_retry()
}
