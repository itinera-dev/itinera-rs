//! The engine both executors share: it runs one journey, emitting its events, and makes its
//! result.

use std::future::Future;

use crate::event::EventBody;
use crate::instance::WorkflowInstance;
use crate::journey::{JourneyResult, JourneyStatus};
use crate::policy::PolicyName;
use crate::step::{StepAttempt, StepDescriptor, StepName};
use crate::workflow::WorkflowDescriptor;

#[cfg(feature = "async")]
mod asynchronous;
mod attempt;
mod conclusion;
mod decision;
mod emitter;
mod end;
mod guard;
mod hooks;
mod inputs;
mod request;
mod retry;

#[cfg(feature = "async")]
pub(crate) use asynchronous::Awaited;
pub(crate) use emitter::{Clock, Emitted, Emitting, Inline};
pub(crate) use guard::Failures;
pub(crate) use hooks::WorkflowPolicies;

use emitter::{Delivery, Emitter};
use end::{Aborted, End};

/// Runs one journey of the instance, with the workflow policies built for it, delivering its
/// events through the dispatcher, whose reporters were guarded by `failures`.
pub(crate) async fn run<I: WorkflowInstance>(
    mut instance: I,
    policies: WorkflowPolicies<I::Workflow, I::Mode>,
    delivery: impl Delivery,
    failures: Failures,
    clock: Clock,
) -> JourneyResult {
    let journey_id = instance.journey_id().clone();
    let descriptor = instance.descriptor().clone();
    let emitter = Emitter::new(journey_id.clone(), descriptor.name(), clock);
    let mut journey = Journey {
        emitter,
        delivery,
        failures,
        step: None,
        interrupted: None,
    };
    let status = match journey.travel(&mut instance, &descriptor, &policies).await {
        Ok(()) => JourneyStatus::Succeeded {
            data: instance.into_data_bag(),
        },
        Err(End::Failed(failing)) => JourneyStatus::Failed {
            failure: failing.failure,
            data: instance.into_data_bag(),
        },
        Err(End::Aborted(aborted)) => journey.abort(*aborted).await,
    };
    JourneyResult { journey_id, status }
}

/// What the scan does after an attempt, when the journey goes on.
#[derive(Debug)]
enum Next {
    /// The step is done: it succeeded or skipped itself.
    Step,
    /// The step is attempted again.
    Attempt,
    /// A hook of the step returned `FinishWorkflow`: the journey succeeds without the steps left.
    Finish(PolicyName),
}

/// One journey while it runs.
struct Journey<D> {
    emitter: Emitter,
    delivery: D,
    failures: Failures,
    /// The step being run, if any.
    step: Option<StepName>,
    /// The abort recorded when a reporter failed on an event a step or hook emitted, while it
    /// ran. It stands whatever the step or hook does afterwards.
    interrupted: Option<Box<Aborted>>,
}

impl<D: Delivery> Journey<D> {
    /// Runs the journey's steps, then calls the workflow hook for how it ended, and reports it.
    ///
    /// It holds the instance only mutably across an await, so that the journey's future is
    /// `Send` whenever the instance is.
    async fn travel<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        descriptor: &WorkflowDescriptor<I::Workflow, I::Mode>,
        policies: &WorkflowPolicies<I::Workflow, I::Mode>,
    ) -> Result<(), End> {
        let initial_keys = instance.data_bag().keys().map(str::to_owned).collect();
        self.emit(EventBody::JourneyStarted { initial_keys })
            .await?;
        let scanned = self.scan(instance, descriptor).await;
        self.step = None;
        match scanned {
            Ok(decided_by) => {
                self.on_workflow_success(instance, descriptor, policies)
                    .await?;
                self.emit(EventBody::JourneySucceeded { decided_by })
                    .await?;
                Ok(())
            }
            Err(End::Failed(failing)) => {
                self.on_workflow_failure(instance, descriptor, policies)
                    .await?;
                self.emit(EventBody::JourneyFailed {
                    step: failing.step,
                    failure: failing.reported.clone(),
                })
                .await?;
                Err(End::Failed(failing))
            }
            Err(aborted) => Err(aborted),
        }
    }

    /// Runs each step in its order until none is left, or one ends the journey. Returns the
    /// policy whose hook finished the journey early, if one did.
    async fn scan<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        descriptor: &WorkflowDescriptor<I::Workflow, I::Mode>,
    ) -> Result<Option<PolicyName>, End> {
        for step in descriptor.steps() {
            self.step = Some(step.name());
            if let Some(policy) = self.run_step(instance, step).await? {
                return Ok(Some(policy));
            }
        }
        Ok(None)
    }

    /// Attempts a step until it is done, or the journey ends. Returns the policy whose hook
    /// finished the journey, if one did.
    async fn run_step<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        step: &StepDescriptor<I::Workflow, I::Mode>,
    ) -> Result<Option<PolicyName>, End> {
        let mut attempt = StepAttempt::first(step.name());
        loop {
            match self.attempt(instance, step, &attempt).await? {
                Next::Attempt => attempt = attempt.next(),
                Next::Step => return Ok(None),
                Next::Finish(policy) => return Ok(Some(policy)),
            }
        }
    }
}

/// Runs a future that never waits to its end, without a runtime.
///
/// A synchronous workflow has nothing to wait for, so its journey is ready the first time it is
/// polled.
pub(crate) fn finish<T>(future: impl Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return value;
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::num::NonZeroU64;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use rstest::rstest;

    use super::*;
    use crate::error::Error;
    use crate::event::{
        self, Event, GiveUpCause, JourneyAbort, JourneyFailure, RequestSource, Source, Timestamp,
    };
    use crate::executor::build_policies;
    use crate::instance::InstanceBuilder;
    use crate::journey::{
        Abort, Contributor, Failure, FailureCause, LastFailure, MissingData, Read as Found,
        Requester,
    };
    use crate::policy::{
        FailWorkflow, HookNeeds, InputAdapter, Lifecycle, OnStepAbnormalTermination, OnStepFailure,
        OnStepRetry, OnStepSuccess, OnSuccess, Requested, RetryCause, StepAbnormalTermination,
        StepFailure, StepHook, StepPolicyDescriptor, StepRetry, StepSuccess,
    };
    use crate::report::{DefaultDispatcher, Dispatcher, Reporter};
    use crate::step::{
        Input, OptionalInput, Outcome, Reason, Resolved, Step, StepDescriptor, StepFactory,
        StepNeeds, StepReporter,
    };
    use crate::value::AnyValue;
    use crate::workflow::{
        AdapterName, InputAdapterDescriptor, WorkflowBuilder, WorkflowDescriptor, WorkflowName,
    };

    pub(super) struct Orders;

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

    pub(super) const CHARGE: StepName = StepName::new("charge");
    const SHIP: StepName = StepName::new("ship");
    pub(super) const AMOUNT: Input<i64> = Input::new("amount");
    const DISCOUNT: OptionalInput<i64> = OptionalInput::new("discount");

    /// What the charge step read, one entry per attempt.
    pub(super) type Read = Arc<Mutex<Vec<(i64, Option<i64>)>>>;

    /// The factory of a step that needs an amount, may have a discount, and records what it read.
    struct Charge {
        read: Read,
    }

    struct Charging<'a> {
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

    /// The factory of a step that can never be built.
    struct Unbuildable;

    impl StepFactory for Unbuildable {
        type Step<'a> = Charging<'a>;

        fn needs(&self) -> StepNeeds {
            StepNeeds::new()
        }

        fn build<'a>(&'a self, _: &mut Resolved<'a>) -> Result<Charging<'a>, Error> {
            Err(Error::msg("no card reader"))
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

    pub(super) fn scripted<S>(script: S) -> StepDescriptor<Orders>
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

    fn noon() -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(1_700_000_000)
    }

    /// Runs the journey to its end, and returns its status and the events it emitted.
    pub(super) fn travel<I: WorkflowInstance>(instance: I) -> (JourneyStatus, Vec<Event>) {
        travel_before(instance, None)
    }

    /// Runs the journey to its end with a recording reporter, before the one given if any, and
    /// returns its status and the events the recording reporter received.
    fn travel_before<I: WorkflowInstance>(
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

    pub(super) fn orders() -> WorkflowBuilder<Orders> {
        WorkflowDescriptor::builder("orders")
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
    fn charged(read: &Read) -> WorkflowBuilder<Orders> {
        orders().step(charge(read))
    }

    /// Runs a journey of the workflow, whose initial data holds an amount of 42.
    pub(super) fn travel_with_amount(
        builder: WorkflowBuilder<Orders>,
    ) -> (JourneyStatus, Vec<Event>) {
        travel(instance(builder).data("amount", 42_i64).create().unwrap())
    }

    /// Runs a journey of the workflow, with no initial data.
    pub(super) fn travel_workflow(builder: WorkflowBuilder<Orders>) -> (JourneyStatus, Vec<Event>) {
        travel(instance(builder).create().unwrap())
    }

    pub(super) fn instance(builder: WorkflowBuilder<Orders>) -> InstanceBuilder<Orders> {
        builder.build().unwrap().instance(Orders)
    }

    pub(super) fn succeed() -> Result<Outcome, Error> {
        Ok(Outcome::success())
    }

    pub(super) fn kinds(events: &[Event]) -> Vec<&'static str> {
        events.iter().map(Event::kind).collect()
    }

    fn reads(read: &Read) -> Vec<(i64, Option<i64>)> {
        read.lock().unwrap().clone()
    }

    fn sequence(event: &Event) -> NonZeroU64 {
        event.sequence
    }

    fn kind_and_step(event: &Event) -> (&'static str, Option<StepName>) {
        let step = match &event.body {
            EventBody::AttemptStarted { step } | EventBody::StepSucceeded { step } => {
                Some(step.step)
            }
            _ => None,
        };
        (event.kind(), step)
    }

    /// Supplies an amount of 7, and nothing else.
    fn pricing(
        _: &Orders,
        got: Requested<'_, Orders, InputAdapter>,
    ) -> Result<Option<AnyValue>, Error> {
        Ok((got.key() == "amount").then(|| AnyValue::new(7_i64)))
    }

    #[test]
    fn steps_run_in_the_order_they_were_added() {
        let workflow = orders()
            .step(StepDescriptor::new(CHARGE, succeed))
            .step(StepDescriptor::new(SHIP, succeed));

        let (_, events) = travel_workflow(workflow);

        let stream: Vec<_> = events.iter().map(kind_and_step).collect();
        assert_eq!(
            stream,
            [
                ("journey_started", None),
                ("attempt_started", Some(CHARGE)),
                ("step_succeeded", Some(CHARGE)),
                ("attempt_started", Some(SHIP)),
                ("step_succeeded", Some(SHIP)),
                ("journey_succeeded", None),
            ]
        );
    }

    #[test]
    fn events_are_numbered_from_one_and_carry_the_journey_the_workflow_and_the_time() {
        let workflow = orders()
            .step(StepDescriptor::new(CHARGE, succeed))
            .id_generator(|_: &Orders, _| Ok("order-7".to_string()));

        let (_, events) = travel_with_amount(workflow);

        let sequences: Vec<u64> = events.iter().map(sequence).map(NonZeroU64::get).collect();
        assert_eq!(sequences, [1, 2, 3, 4]);
        for event in &events {
            assert_eq!(event.journey_id.to_string(), "order-7");
            assert_eq!(event.workflow, WorkflowName::from("orders"));
            assert_eq!(event.timestamp, Timestamp::from(noon()));
        }
        let Some(EventBody::JourneyStarted { initial_keys }) = events.first().map(|e| &e.body)
        else {
            panic!("the first event is not journey_started");
        };
        assert_eq!(initial_keys, &["amount"]);
    }

    #[test]
    fn a_step_is_built_with_the_inputs_it_declares_read_from_the_data_bag() {
        let read = Read::default();
        let journey = instance(charged(&read))
            .data("amount", 42_i64)
            .data("discount", 5_i64);

        let (status, events) = travel(journey.create().unwrap());

        assert!(matches!(status, JourneyStatus::Succeeded { .. }));
        assert_eq!(reads(&read), [(42, Some(5))]);
        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                "step_succeeded",
                "journey_succeeded"
            ]
        );
    }

    #[test]
    fn an_optional_input_without_a_value_is_absent_and_reported() {
        let read = Read::default();
        let journey = instance(charged(&read)).data("amount", 42_i64);

        let (_, events) = travel(journey.create().unwrap());

        assert_eq!(reads(&read), [(42, None)]);
        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                "optional_input_absent",
                "step_succeeded",
                "journey_succeeded"
            ]
        );
        let Some(EventBody::OptionalInputAbsent { key, requester }) =
            events.get(2).map(|e| &e.body)
        else {
            panic!("the third event is not optional_input_absent");
        };
        assert_eq!(key, "discount");
        assert_eq!(requester, &RequestSource::Step(StepAttempt::first(CHARGE)));
    }

    #[test]
    fn a_required_input_without_a_value_aborts_the_journey_before_the_step_is_built() {
        let read = Read::default();

        let (status, events) = travel_workflow(charged(&read));

        let JourneyStatus::Aborted(Abort::RequiredDataMissing(MissingData::Key { key, requester })) =
            status
        else {
            panic!("the journey was not aborted for missing data: {status:?}");
        };
        assert_eq!(key, "amount");
        assert_eq!(requester, Requester::Step);
        assert!(reads(&read).is_empty());
        assert_eq!(
            kinds(&events),
            ["journey_started", "attempt_started", "journey_aborted"]
        );
    }

    type Data = fn(InstanceBuilder<Orders>) -> InstanceBuilder<Orders>;

    fn amount_as_i32(journey: InstanceBuilder<Orders>) -> InstanceBuilder<Orders> {
        journey.data("amount", 42_i32)
    }

    fn discount_as_text(journey: InstanceBuilder<Orders>) -> InstanceBuilder<Orders> {
        journey
            .data("amount", 42_i64)
            .data("discount", "five".to_string())
    }

    #[rstest]
    #[case::a_narrower_integer_for_a_required_input(amount_as_i32, "amount")]
    #[case::text_for_an_optional_input(discount_as_text, "discount")]
    fn a_value_of_another_type_aborts_the_journey_with_wrong_type(
        #[case] data: Data,
        #[case] expected: &str,
    ) {
        let read = Read::default();

        let (status, _) = travel(data(instance(charged(&read))).create().unwrap());

        let JourneyStatus::Aborted(Abort::WrongType { key, requester }) = status else {
            panic!("the journey was not aborted for a wrong type: {status:?}");
        };
        assert_eq!(key, expected);
        assert_eq!(requester, Requester::Step);
        assert!(reads(&read).is_empty());
    }

    #[test]
    fn an_input_adapter_supplies_an_input_before_the_data_bag_is_read() {
        let read = Read::default();
        let workflow =
            charged(&read).input_adapter(InputAdapterDescriptor::new("pricing", CHARGE, pricing));

        let (_, events) = travel_with_amount(workflow);

        assert_eq!(reads(&read), [(7, None)]);
        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                "input_adapter_supplied",
                "optional_input_absent",
                "step_succeeded",
                "journey_succeeded"
            ]
        );
    }

    #[test]
    fn an_input_adapter_that_fails_aborts_the_journey_as_its_step_could_not_be_built() {
        let read = Read::default();
        let failing = InputAdapterDescriptor::new("pricing", CHARGE, |_: &Orders, _| {
            Err(Error::msg("no price list"))
        });

        let (status, events) = travel_workflow(charged(&read).input_adapter(failing));

        let JourneyStatus::Aborted(Abort::StepCouldNotBeBuilt(error)) = status else {
            panic!("the journey was not aborted as its step could not be built: {status:?}");
        };
        assert_eq!(error.to_string(), "no price list");
        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                "input_adapter_failed",
                "journey_aborted"
            ]
        );
    }

    #[test]
    fn a_value_of_another_type_from_an_input_adapter_aborts_the_journey_with_wrong_type() {
        let read = Read::default();
        let wrong = InputAdapterDescriptor::new("pricing", CHARGE, |_: &Orders, _| {
            Ok(Some(AnyValue::new("seven".to_string())))
        });

        let (status, events) = travel_workflow(charged(&read).input_adapter(wrong));

        let JourneyStatus::Aborted(Abort::WrongType { key, requester }) = status else {
            panic!("the journey was not aborted for a wrong type: {status:?}");
        };
        assert_eq!(key, "amount");
        assert_eq!(
            requester,
            Requester::Adapter {
                adapter: AdapterName::from("pricing")
            }
        );
        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                "input_adapter_supplied",
                "journey_aborted"
            ]
        );
    }

    fn wrong_type_naming_the_step(status: &JourneyStatus) -> bool {
        matches!(
            status,
            JourneyStatus::Aborted(Abort::WrongType { key, requester: Requester::Step })
                if key == "amount"
        )
    }

    fn missing_data_naming_the_step(status: &JourneyStatus) -> bool {
        matches!(
            status,
            JourneyStatus::Aborted(Abort::RequiredDataMissing(MissingData::Key {
                key,
                requester: Requester::Step
            })) if key == "amount"
        )
    }

    #[rstest]
    #[case::a_value_of_another_type_in_the_data_bag(amount_as_i32, wrong_type_naming_the_step)]
    #[case::no_value_in_the_data_bag(std::convert::identity, missing_data_naming_the_step)]
    fn an_input_adapter_that_returned_nothing_is_not_named_in_the_abort(
        #[case] data: Data,
        #[case] aborted: fn(&JourneyStatus) -> bool,
    ) {
        let read = Read::default();
        let silent = InputAdapterDescriptor::new("pricing", CHARGE, |_: &Orders, _| Ok(None));
        let workflow = charged(&read).input_adapter(silent);

        let (status, events) = travel(data(instance(workflow)).create().unwrap());

        assert!(aborted(&status), "{status:?}");
        assert!(reads(&read).is_empty());
        assert_eq!(
            kinds(&events),
            ["journey_started", "attempt_started", "journey_aborted"]
        );
    }

    const PRICE: Input<i64> = Input::new("price");
    const QUANTITY: OptionalInput<i64> = OptionalInput::new("quantity");

    /// Supplies the amount: the price from the workflow, times the quantity when there is one.
    fn priced(
        _: &Orders,
        mut got: Requested<'_, Orders, InputAdapter>,
    ) -> Result<Option<AnyValue>, Error> {
        let price = got.from_workflow(&PRICE)?;
        let amount = price * got.optional_from_workflow(&QUANTITY)?.unwrap_or(1);
        Ok((got.key() == "amount").then(|| AnyValue::new(amount)))
    }

    /// The charge step, adapted by `priced`.
    fn charged_at_the_price(read: &Read) -> WorkflowBuilder<Orders> {
        let pricing = InputAdapterDescriptor::new("pricing", CHARGE, priced).needing(
            HookNeeds::new()
                .from_workflow(&PRICE)
                .optional_from_workflow(&QUANTITY),
        );
        charged(read).input_adapter(pricing)
    }

    #[test]
    fn an_input_adapter_receives_the_data_from_the_workflow_it_requests() {
        let read = Read::default();

        let journey = instance(charged_at_the_price(&read))
            .data("price", 6_i64)
            .data("quantity", 2_i64);
        let (_, events) = travel(journey.create().unwrap());

        assert_eq!(reads(&read), [(12, None)]);
        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                "input_adapter_supplied",
                "optional_input_absent",
                "step_succeeded",
                "journey_succeeded"
            ]
        );
    }

    #[test]
    fn an_input_adapters_absent_optional_data_is_reported_before_what_it_supplied() {
        let read = Read::default();

        let journey = instance(charged_at_the_price(&read)).data("price", 7_i64);
        let (_, events) = travel(journey.create().unwrap());

        assert_eq!(reads(&read), [(7, None)]);
        let Some(Event {
            body: EventBody::OptionalInputAbsent { key, requester },
            ..
        }) = events.get(2)
        else {
            panic!("no optional_input_absent after attempt_started: {events:?}");
        };
        assert_eq!(key, "quantity");
        assert_eq!(
            *requester,
            RequestSource::Adapter {
                adapter: AdapterName::from("pricing"),
                step: StepAttempt::first(CHARGE),
            }
        );
        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                "optional_input_absent",
                "input_adapter_supplied",
                "optional_input_absent",
                "optional_input_absent",
                "step_succeeded",
                "journey_succeeded"
            ]
        );
    }

    fn price_as_text(journey: InstanceBuilder<Orders>) -> InstanceBuilder<Orders> {
        journey.data("price", "six".to_string())
    }

    fn wrong_type_naming_the_adapter(status: &JourneyStatus) -> bool {
        matches!(
            status,
            JourneyStatus::Aborted(Abort::WrongType { key, requester: Requester::Adapter { adapter } })
                if key == "price" && *adapter == AdapterName::from("pricing")
        )
    }

    fn missing_data_naming_the_adapter(status: &JourneyStatus) -> bool {
        matches!(
            status,
            JourneyStatus::Aborted(Abort::RequiredDataMissing(MissingData::Key {
                key,
                requester: Requester::Adapter { adapter }
            })) if key == "price" && *adapter == AdapterName::from("pricing")
        )
    }

    #[rstest]
    #[case::a_value_of_another_type_in_the_data_bag(price_as_text, wrong_type_naming_the_adapter)]
    #[case::no_value_in_the_data_bag(std::convert::identity, missing_data_naming_the_adapter)]
    fn an_input_adapters_required_data_from_the_workflow_aborts_the_journey_before_it_runs(
        #[case] data: Data,
        #[case] aborted: fn(&JourneyStatus) -> bool,
    ) {
        let read = Read::default();

        let (status, events) = travel(
            data(instance(charged_at_the_price(&read)))
                .create()
                .unwrap(),
        );

        assert!(aborted(&status), "{status:?}");
        assert!(reads(&read).is_empty());
        assert_eq!(
            kinds(&events),
            ["journey_started", "attempt_started", "journey_aborted"]
        );
        assert_eq!(
            events.last().and_then(aborting_requester),
            Some(&event::Requester::Adapter {
                adapter: AdapterName::from("pricing"),
                step: CHARGE,
            })
        );
    }

    /// Who made the request that aborted the journey, as `journey_aborted` names it.
    fn aborting_requester(event: &Event) -> Option<&event::Requester> {
        match &event.body {
            EventBody::JourneyAborted {
                abort:
                    JourneyAbort::RequiredDataMissing {
                        missing: event::MissingData::Key { requester, .. },
                    }
                    | JourneyAbort::WrongType { requester, .. },
            } => Some(requester),
            _ => None,
        }
    }

    /// The names of the steps an input adapter was told it supplies, in order.
    type Names = Arc<Mutex<Vec<StepName>>>;

    /// Records the name of the step it supplies an input to, and supplies nothing.
    fn naming(
        names: &Names,
    ) -> impl for<'a> Fn(
        &'a Orders,
        Requested<'a, Orders, InputAdapter>,
    ) -> Result<Option<AnyValue>, Error>
    + Send
    + Sync
    + 'static {
        let names = Arc::clone(names);
        move |_, got| {
            names.lock().unwrap().push(got.step_name());
            Ok(None)
        }
    }

    #[test]
    fn an_input_adapter_is_told_the_step_it_supplies_for_each_of_its_inputs() {
        let names = Names::default();
        let read = Read::default();
        let ship = Charge {
            read: Arc::clone(&read),
        };
        let workflow = charged(&read)
            .step(StepDescriptor::new(SHIP, ship))
            .input_adapter(
                InputAdapterDescriptor::new("naming", CHARGE, naming(&names)).step(SHIP),
            );

        travel(instance(workflow).data("amount", 42_i64).create().unwrap());

        assert_eq!(*names.lock().unwrap(), [CHARGE, CHARGE, SHIP, SHIP]);
    }

    /// Supplies the amount as twice the base read through its access to the data bag.
    fn doubled(
        _: &Orders,
        mut got: Requested<'_, Orders, InputAdapter>,
    ) -> Result<Option<AnyValue>, Error> {
        let base = got.data_bag()?.read::<i64>("base");
        match (got.key(), base) {
            ("amount", Found::Present(base)) => Ok(Some(AnyValue::new(base * 2))),
            _ => Ok(None),
        }
    }

    #[test]
    fn an_input_adapter_reads_the_data_bag_without_events() {
        let read = Read::default();
        let workflow = charged(&read).input_adapter(
            InputAdapterDescriptor::new("pricing", CHARGE, doubled)
                .needing(HookNeeds::new().data_bag()),
        );

        let (_, events) = travel(instance(workflow).data("base", 5_i64).create().unwrap());

        assert_eq!(reads(&read), [(10, None)]);
        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                "input_adapter_supplied",
                "optional_input_absent",
                "step_succeeded",
                "journey_succeeded"
            ]
        );
    }

    #[test]
    fn an_input_adapter_that_did_not_declare_the_data_bag_cannot_read_it() {
        let read = Read::default();
        let workflow =
            charged(&read).input_adapter(InputAdapterDescriptor::new("pricing", CHARGE, doubled));

        let (status, _) = travel(instance(workflow).data("base", 5_i64).create().unwrap());

        assert!(matches!(
            status,
            JourneyStatus::Aborted(Abort::StepCouldNotBeBuilt(_))
        ));
    }

    #[test]
    fn a_step_whose_factory_fails_aborts_the_journey_as_it_could_not_be_built() {
        let workflow = orders().step(StepDescriptor::new(CHARGE, Unbuildable));

        let (status, events) = travel_workflow(workflow);

        assert!(matches!(
            status,
            JourneyStatus::Aborted(Abort::StepCouldNotBeBuilt(_))
        ));
        assert_eq!(
            kinds(&events),
            ["journey_started", "attempt_started", "journey_aborted"]
        );
    }

    pub(super) fn declined() -> Result<Outcome, Error> {
        Ok(Outcome::failure(Reason::new("declined")))
    }

    pub(super) fn timed_out() -> Result<Outcome, Error> {
        Ok(Outcome::retriable_failure(Reason::new("timeout")))
    }

    pub(super) fn crashed() -> Result<Outcome, Error> {
        Err(Error::msg("the gateway crashed"))
    }

    #[rstest]
    #[case::a_failure(declined, "step_failed", FailureCause::Failure)]
    #[case::a_retriable_failure(timed_out, "step_failed", FailureCause::RetriesExhausted)]
    #[case::an_abnormal_termination(
        crashed,
        "step_abnormal_termination",
        FailureCause::AbnormalTermination
    )]
    fn without_a_retry_budget_a_failed_attempt_is_the_steps_last_and_fails_the_journey(
        #[case] ends: fn() -> Result<Outcome, Error>,
        #[case] fact: &str,
        #[case] cause: FailureCause,
    ) {
        let workflow = orders()
            .step(StepDescriptor::new(CHARGE, ends))
            .step(StepDescriptor::new(SHIP, succeed));

        let (status, events) = travel_workflow(workflow);

        let JourneyStatus::Failed { failure, .. } = status else {
            panic!("the journey did not fail: {status:?}");
        };
        assert_eq!(failure.cause(), cause);
        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                fact,
                "step_given_up",
                "journey_failed"
            ]
        );
        let Some(EventBody::JourneyFailed { step, failure }) = events.last().map(|e| &e.body)
        else {
            panic!("the last event is not journey_failed");
        };
        assert_eq!((*step, failure.cause()), (CHARGE, cause));
    }

    #[test]
    fn a_skipped_step_discards_its_contributions_and_the_journey_goes_on() {
        let workflow = orders()
            .step(StepDescriptor::new(CHARGE, || Ok(Outcome::skipped())))
            .step(StepDescriptor::new(SHIP, succeed));

        let (status, events) = travel_workflow(workflow);

        assert!(matches!(status, JourneyStatus::Succeeded { .. }));
        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                "step_skipped",
                "contributions_discarded",
                "attempt_started",
                "step_succeeded",
                "journey_succeeded"
            ]
        );
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

    fn attempt_of(event: &Event) -> Option<u32> {
        match &event.body {
            EventBody::AttemptStarted { step } => Some(step.attempt.get()),
            _ => None,
        }
    }

    fn retry_cause(event: &Event) -> Option<RetryCause> {
        match &event.body {
            EventBody::StepRetrying { cause, .. } => Some(*cause),
            _ => None,
        }
    }

    #[test]
    fn a_retriable_failure_is_attempted_again_while_the_budget_allows() {
        let workflow = orders().step(attempting(&[timed_out, timed_out]).retry_budget(2));

        let (status, events) = travel_workflow(workflow);

        assert_eq!(data_of(&status), [("attempt".to_string(), 3)]);
        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                "step_failed",
                "step_retrying",
                "attempt_started",
                "step_failed",
                "step_retrying",
                "attempt_started",
                "step_succeeded",
                "contribution_committed",
                "journey_succeeded"
            ]
        );
        let attempts: Vec<_> = events.iter().filter_map(attempt_of).collect();
        assert_eq!(attempts, [1, 2, 3]);
        let causes: Vec<_> = events.iter().filter_map(retry_cause).collect();
        assert_eq!(causes, [RetryCause::RetriableFailure; 2]);
    }

    #[test]
    fn a_step_whose_budget_is_spent_is_given_up_with_its_last_failure() {
        let workflow = orders()
            .step(attempting(&[timed_out, timed_out, succeed]).retry_budget(1))
            .step(StepDescriptor::new(SHIP, succeed));

        let (status, events) = travel_workflow(workflow);

        let JourneyStatus::Failed { failure, data } = status else {
            panic!("the journey did not fail: {status:?}");
        };
        assert!(matches!(
            failure,
            Failure::RetriesExhausted(LastFailure::Reason(reason)) if reason.code() == "timeout"
        ));
        assert!(data.get("attempt").is_none());
        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                "step_failed",
                "step_retrying",
                "attempt_started",
                "step_failed",
                "step_given_up",
                "journey_failed"
            ]
        );
        assert!(matches!(
            events.get(6).map(|e| &e.body),
            Some(EventBody::StepGivenUp { cause: GiveUpCause::RetriesExhausted, step })
                if step.attempt.get() == 2
        ));
    }

    #[rstest]
    #[case::when_the_step_allows_it(
        attempting(&[crashed]).retry_budget(1).abnormal_termination_retriable(),
        "step_retrying"
    )]
    #[case::not_when_the_step_does_not(attempting(&[crashed]).retry_budget(1), "step_given_up")]
    fn an_abnormal_termination_is_retried_only_when_the_step_allows_it(
        #[case] step: StepDescriptor<Orders>,
        #[case] decision: &str,
    ) {
        let (_, events) = travel_workflow(orders().step(step));

        assert_eq!(
            kinds(&events).get(..4),
            Some(
                &[
                    "journey_started",
                    "attempt_started",
                    "step_abnormal_termination",
                    decision
                ][..]
            )
        );
    }

    #[test]
    fn an_abnormal_termination_is_retried_with_its_own_cause() {
        let step = attempting(&[crashed])
            .retry_budget(1)
            .abnormal_termination_retriable();

        let (status, events) = travel_workflow(orders().step(step));

        assert!(matches!(status, JourneyStatus::Succeeded { .. }));
        let causes: Vec<_> = events.iter().filter_map(retry_cause).collect();
        assert_eq!(causes, [RetryCause::AbnormalTermination]);
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

    fn audit() -> PolicyName {
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
    struct Hooked {
        status: JourneyStatus,
        events: Vec<Event>,
        calls: Vec<(StepHook, usize)>,
    }

    /// Runs a journey of a workflow whose first step has the policy "audit", whose hook `hook`
    /// returns `lifecycle`, and whose second step, if any, has no policy.
    fn travel_hooked(
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

    fn fail_workflow() -> Lifecycle {
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

    fn failed_by_the_hook(hooked: &Hooked, hook: StepHook) {
        let JourneyStatus::Failed { failure, .. } = &hooked.status else {
            panic!("the journey did not fail: {:?}", hooked.status);
        };
        assert!(matches!(failure, Failure::FailWorkflow(reason) if reason.code() == "fraud"));
        assert_eq!(
            hooked.events.last().and_then(decided_by),
            Some((audit(), hook))
        );
    }

    #[test]
    fn finish_workflow_from_on_step_success_succeeds_the_journey_without_the_steps_left() {
        let hooked = travel_hooked(
            attempting(&[]),
            Some(StepDescriptor::new(SHIP, crashed)),
            StepHook::OnStepSuccess,
            Lifecycle::FinishWorkflow,
        );

        assert_eq!(data_of(&hooked.status), [("attempt".to_string(), 1)]);
        assert_eq!(
            kinds(&hooked.events),
            [
                "journey_started",
                "attempt_started",
                "step_succeeded",
                "contribution_committed",
                "hook_called",
                "journey_succeeded"
            ]
        );
        assert!(matches!(
            hooked.events.last().map(|e| &e.body),
            Some(EventBody::JourneySucceeded { decided_by: Some(policy) }) if *policy == audit()
        ));
    }

    #[test]
    fn fail_workflow_from_on_step_success_fails_the_journey_after_committing_the_contributions() {
        let hooked = travel_hooked(
            attempting(&[]),
            Some(StepDescriptor::new(SHIP, succeed)),
            StepHook::OnStepSuccess,
            fail_workflow(),
        );

        failed_by_the_hook(&hooked, StepHook::OnStepSuccess);
        assert_eq!(data_of(&hooked.status), [("attempt".to_string(), 1)]);
        assert_eq!(
            kinds(&hooked.events),
            [
                "journey_started",
                "attempt_started",
                "step_succeeded",
                "contribution_committed",
                "hook_called",
                "journey_failed"
            ]
        );
    }

    #[rstest]
    #[case::on_step_retry(
        attempting(&[timed_out]).retry_budget(1),
        StepHook::OnStepRetry,
        "step_failed"
    )]
    #[case::on_step_abnormal_termination(
        attempting(&[crashed]).retry_budget(1).abnormal_termination_retriable(),
        StepHook::OnStepAbnormalTermination,
        "step_abnormal_termination"
    )]
    fn fail_workflow_from_a_hook_before_the_decision_gives_the_step_up_without_on_step_failure(
        #[case] step: StepDescriptor<Orders>,
        #[case] hook: StepHook,
        #[case] fact: &str,
    ) {
        let hooked = travel_hooked(step, None, hook, fail_workflow());

        failed_by_the_hook(&hooked, hook);
        assert_eq!(
            kinds(&hooked.events),
            [
                "journey_started",
                "attempt_started",
                fact,
                "hook_called",
                "step_given_up",
                "journey_failed"
            ]
        );
        assert!(matches!(
            hooked.events.get(4).map(|e| &e.body),
            Some(EventBody::StepGivenUp {
                cause: GiveUpCause::FailWorkflow { decided_by, .. },
                ..
            }) if decided_by.policy == audit() && decided_by.hook.step_hook() == hook
        ));
        assert_eq!(hooked.calls, [(hook, 3)]);
    }

    #[test]
    fn fail_workflow_from_on_step_failure_gives_the_journey_its_reason_after_the_step_is_given_up()
    {
        let hooked = travel_hooked(
            attempting(&[declined]),
            None,
            StepHook::OnStepFailure,
            fail_workflow(),
        );

        failed_by_the_hook(&hooked, StepHook::OnStepFailure);
        assert_eq!(
            kinds(&hooked.events),
            [
                "journey_started",
                "attempt_started",
                "step_failed",
                "step_given_up",
                "hook_called",
                "journey_failed"
            ]
        );
        assert!(matches!(
            hooked.events.get(3).map(|e| &e.body),
            Some(EventBody::StepGivenUp {
                cause: GiveUpCause::Failure,
                ..
            })
        ));
        assert_eq!(hooked.calls, [(StepHook::OnStepFailure, 4)]);
    }

    #[rstest]
    #[case::a_retriable_failure_with_budget_left(
        attempting(&[timed_out]).retry_budget(1),
        &[(StepHook::OnStepRetry, 3), (StepHook::OnStepSuccess, 8)]
    )]
    #[case::a_retriable_failure_with_the_budget_spent(
        attempting(&[timed_out]),
        &[(StepHook::OnStepFailure, 4)]
    )]
    #[case::a_failure(attempting(&[declined]).retry_budget(1), &[(StepHook::OnStepFailure, 4)])]
    #[case::an_abnormal_termination_the_step_does_not_retry(
        attempting(&[crashed]).retry_budget(1),
        &[(StepHook::OnStepAbnormalTermination, 3), (StepHook::OnStepFailure, 5)]
    )]
    #[case::an_abnormal_termination_the_step_retries(
        attempting(&[crashed]).retry_budget(1).abnormal_termination_retriable(),
        &[
            (StepHook::OnStepAbnormalTermination, 3),
            (StepHook::OnStepRetry, 4),
            (StepHook::OnStepSuccess, 9)
        ]
    )]
    #[case::an_abnormal_termination_with_the_budget_spent(
        attempting(&[crashed]).abnormal_termination_retriable(),
        &[(StepHook::OnStepAbnormalTermination, 3), (StepHook::OnStepFailure, 5)]
    )]
    #[case::a_skip(attempting(&[skips]), &[])]
    fn step_hooks_are_called_after_each_attempt_in_order_around_the_step_decision(
        #[case] step: StepDescriptor<Orders>,
        #[case] calls: &[(StepHook, usize)],
    ) {
        let hooked = travel_hooked(
            step,
            None,
            StepHook::OnStepSuccess,
            Lifecycle::FinishWorkflow,
        );

        assert_eq!(hooked.calls, calls);
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

    fn key_of(event: &Event) -> Option<&str> {
        match &event.body {
            EventBody::ContributionCommitted { key, .. }
            | EventBody::DataOverwritten { key, .. } => Some(key),
            _ => None,
        }
    }

    #[test]
    fn a_successful_attempts_contributions_are_committed_in_order_reporting_what_they_overwrote() {
        let workflow = orders().step(scripted(|contributor, _| {
            contributor.contribute("receipt", 1_i64);
            contributor.contribute("amount", 40_i64);
            contributor.contribute("receipt", 2_i64);
            Ok(Outcome::success())
        }));

        let (status, events) = travel_with_amount(workflow);

        assert_eq!(
            data_of(&status),
            [("amount".to_string(), 40), ("receipt".to_string(), 2)]
        );
        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                "step_succeeded",
                "contribution_committed",
                "contribution_committed",
                "data_overwritten",
                "journey_succeeded"
            ]
        );
        let keys: Vec<_> = events.iter().filter_map(key_of).collect();
        assert_eq!(keys, ["receipt", "amount", "amount"]);
        let Some(EventBody::ContributionCommitted { source, .. }) = events.get(3).map(|e| &e.body)
        else {
            panic!("the fourth event is not contribution_committed");
        };
        assert_eq!(source, &Source::Step(StepAttempt::first(CHARGE)));
    }

    fn skips() -> Result<Outcome, Error> {
        Ok(Outcome::skipped())
    }

    #[rstest]
    #[case::a_skip(skips)]
    #[case::a_failure(declined)]
    #[case::a_retriable_failure(timed_out)]
    #[case::an_abnormal_termination(crashed)]
    fn an_attempt_that_does_not_succeed_commits_nothing(
        #[case] ends: fn() -> Result<Outcome, Error>,
    ) {
        let workflow = orders().step(scripted(move |contributor, _| {
            contributor.contribute("receipt", 1_i64);
            ends()
        }));

        let (status, events) = travel_with_amount(workflow);

        assert_eq!(data_of(&status), [("amount".to_string(), 42)]);
        assert!(!kinds(&events).contains(&"contribution_committed"));
    }

    #[test]
    fn a_step_emits_its_own_events_stamped_with_its_attempt_before_its_outcome() {
        let workflow = orders().step(scripted(|_, reporter| {
            reporter.info("charging")?;
            reporter.warning_with("slow", 300_i64)?;
            reporter.error("no receipt")?;
            Ok(Outcome::success())
        }));

        let (_, events) = travel_workflow(workflow);

        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                "step_info",
                "step_warning",
                "step_error",
                "step_succeeded",
                "journey_succeeded"
            ]
        );
        let Some(EventBody::StepWarning {
            step,
            message,
            data,
        }) = events.get(3).map(|e| &e.body)
        else {
            panic!("the fourth event is not step_warning");
        };
        assert_eq!(step, &StepAttempt::first(CHARGE));
        assert_eq!(message, "slow");
        assert_eq!(
            data.as_ref().and_then(AnyValue::downcast_ref),
            Some(&300_i64)
        );
    }

    #[test]
    fn a_reporter_failing_on_a_step_event_aborts_the_journey_whatever_the_step_does_next() {
        let emitted = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&emitted);
        let workflow = orders().step(scripted(move |contributor, reporter| {
            let first = reporter.info("charging");
            let second = reporter.warning("still charging");
            seen.lock().unwrap().extend([first.is_ok(), second.is_ok()]);
            contributor.contribute("receipt", 1_i64);
            Ok(Outcome::success())
        }));
        let (status, events) = travel_before(
            instance(workflow).create().unwrap(),
            Some(failing_on("step_info")),
        );

        let JourneyStatus::Aborted(Abort::ReporterFailed(error)) = status else {
            panic!("the journey was not aborted as a reporter failed: {status:?}");
        };
        assert_eq!(error.to_string(), "failed on step_info");
        assert_eq!(*emitted.lock().unwrap(), [false, false]);
        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                "step_info",
                "journey_aborted"
            ]
        );
        let Some(EventBody::JourneyAborted { abort }) = events.last().map(|e| &e.body) else {
            panic!("the last event is not journey_aborted");
        };
        assert_eq!(abort.step(), Some(CHARGE));
    }

    #[test]
    fn a_step_that_takes_a_handle_it_did_not_declare_cannot_be_built() {
        let workflow = orders().step(StepDescriptor::new(CHARGE, Undeclaring));

        let (status, _) = travel_workflow(workflow);

        let JourneyStatus::Aborted(Abort::StepCouldNotBeBuilt(error)) = status else {
            panic!("the journey was not aborted as its step could not be built: {status:?}");
        };
        assert_eq!(
            error.to_string(),
            "the step does not declare a step reporter, or took it already"
        );
    }

    #[test]
    fn a_reporter_failing_on_an_event_emitted_while_the_step_is_built_aborts_the_journey() {
        let workflow = orders().step(StepDescriptor::new(CHARGE, Preparing));

        let (status, events) = travel_before(
            instance(workflow).create().unwrap(),
            Some(failing_on("step_info")),
        );

        assert!(matches!(
            status,
            JourneyStatus::Aborted(Abort::ReporterFailed(_))
        ));
        let Some(EventBody::JourneyAborted { abort }) = events.last().map(|e| &e.body) else {
            panic!("the last event is not journey_aborted");
        };
        assert_eq!(abort.step(), Some(CHARGE));
    }

    /// The factory of a step that emits while it is built, and fails to build when interrupted.
    struct Preparing;

    impl StepFactory for Preparing {
        type Step<'a> = StepReporter<'a>;

        fn needs(&self) -> StepNeeds {
            StepNeeds::new().reporter()
        }

        fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<StepReporter<'a>, Error> {
            let mut reporter = got.reporter()?;
            reporter.info("preparing")?;
            Ok(reporter)
        }
    }

    /// The factory of a step that takes a reporter it does not declare.
    struct Undeclaring;

    impl StepFactory for Undeclaring {
        type Step<'a> = StepReporter<'a>;

        fn needs(&self) -> StepNeeds {
            StepNeeds::new()
        }

        fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<StepReporter<'a>, Error> {
            got.reporter()
        }
    }

    impl Step for StepReporter<'_> {
        fn run(self) -> Result<Outcome, Error> {
            Ok(Outcome::success())
        }
    }

    #[cfg(feature = "async")]
    mod asynchronous {
        use super::*;
        use crate::mode::Asynchronous;
        use crate::policy::{
            AsyncOnStepSuccess, AsyncOnWorkflowFailure, HookNeeds, WorkflowFailure,
            WorkflowPolicyDescriptor,
        };
        use crate::step::{AsyncStep, AsyncStepFactory, AsyncStepReporter};

        fn async_orders() -> WorkflowBuilder<Orders, Asynchronous> {
            WorkflowDescriptor::async_builder("orders")
        }

        impl AsyncStepFactory for Charge {
            type Step<'a> = Charging<'a>;

            fn needs(&self) -> StepNeeds {
                Self::inputs()
            }

            fn build<'a>(
                &'a self,
                got: &mut Resolved<'a, Asynchronous>,
            ) -> Result<Charging<'a>, Error> {
                self.charging(got)
            }
        }

        impl AsyncStep for Charging<'_> {
            async fn run(self) -> Result<Outcome, Error> {
                self.record()
            }
        }

        #[test]
        fn an_asynchronous_step_is_built_with_its_inputs_and_awaited() {
            let read = Read::default();
            let charge = Charge {
                read: Arc::clone(&read),
            };
            let workflow = async_orders()
                .step(StepDescriptor::new_async(CHARGE, charge))
                .build()
                .unwrap();
            let journey = workflow.instance(Orders).data("amount", 42_i64);

            let (status, _) = travel(journey.create().unwrap());

            assert!(matches!(status, JourneyStatus::Succeeded { .. }));
            assert_eq!(reads(&read), [(42, None)]);
        }

        struct Announcing;

        struct Announcer<'a> {
            reporter: AsyncStepReporter<'a>,
        }

        impl AsyncStepFactory for Announcing {
            type Step<'a> = Announcer<'a>;

            fn needs(&self) -> StepNeeds {
                StepNeeds::new().reporter()
            }

            fn build<'a>(
                &'a self,
                got: &mut Resolved<'a, Asynchronous>,
            ) -> Result<Announcer<'a>, Error> {
                Ok(Announcer {
                    reporter: got.reporter()?,
                })
            }
        }

        impl AsyncStep for Announcer<'_> {
            async fn run(mut self) -> Result<Outcome, Error> {
                self.reporter.info_with("charging", 42_i64).await?;
                Ok(Outcome::success())
            }
        }

        #[test]
        fn an_asynchronous_step_awaits_its_own_events() {
            let workflow = async_orders()
                .step(StepDescriptor::new_async(CHARGE, Announcing))
                .build()
                .unwrap();

            let (_, events) = travel(workflow.instance(Orders).create().unwrap());

            assert_eq!(
                kinds(&events),
                [
                    "journey_started",
                    "attempt_started",
                    "step_info",
                    "step_succeeded",
                    "journey_succeeded"
                ]
            );
        }

        #[test]
        fn a_reporter_failing_on_an_asynchronous_steps_event_aborts_the_journey() {
            let workflow = async_orders()
                .step(StepDescriptor::new_async(CHARGE, Announcing))
                .build()
                .unwrap();

            let (status, events) = travel_before(
                workflow.instance(Orders).create().unwrap(),
                Some(failing_on("step_info")),
            );

            assert!(matches!(
                status,
                JourneyStatus::Aborted(Abort::ReporterFailed(_))
            ));
            assert_eq!(
                kinds(&events),
                [
                    "journey_started",
                    "attempt_started",
                    "step_info",
                    "journey_aborted"
                ]
            );
        }

        /// `on step success` of an asynchronous workflow, which awaits its own event and
        /// finishes the journey.
        struct Announce;

        impl AsyncOnStepSuccess<Orders> for Announce {
            fn needs() -> HookNeeds<StepSuccess> {
                HookNeeds::new().reporter()
            }

            async fn on_step_success(
                &self,
                mut got: Requested<'_, Orders, StepSuccess, Asynchronous>,
            ) -> Result<Option<OnSuccess>, Error> {
                got.reporter()?.info("charged").await?;
                Ok(Some(OnSuccess::FinishWorkflow))
            }
        }

        #[test]
        fn an_asynchronous_hook_is_awaited_and_decides_as_a_synchronous_one() {
            let announce =
                StepPolicyDescriptor::new_async("announce", || Announce).on_step_success();
            let workflow = async_orders()
                .step(
                    StepDescriptor::new_async(CHARGE, async || Ok(Outcome::success()))
                        .policy(announce),
                )
                .step(StepDescriptor::new_async(SHIP, async || {
                    Err(Error::msg("never run"))
                }))
                .build()
                .unwrap();

            let (status, events) = travel(workflow.instance(Orders).create().unwrap());

            assert!(matches!(status, JourneyStatus::Succeeded { .. }));
            assert_eq!(
                kinds(&events),
                [
                    "journey_started",
                    "attempt_started",
                    "step_succeeded",
                    "journey_info",
                    "hook_called",
                    "journey_succeeded"
                ]
            );
        }

        /// `on workflow failure` of an asynchronous workflow, which awaits its own event.
        struct Regret;

        impl AsyncOnWorkflowFailure<Orders> for Regret {
            fn needs() -> HookNeeds<WorkflowFailure> {
                HookNeeds::new().reporter()
            }

            async fn on_workflow_failure(
                &self,
                mut got: Requested<'_, Orders, WorkflowFailure, Asynchronous>,
            ) -> Result<(), Error> {
                got.reporter()?.warning("declined").await?;
                Ok(())
            }
        }

        #[test]
        fn an_asynchronous_workflow_hook_is_awaited_before_the_journey_is_reported() {
            let workflow = async_orders()
                .policy(
                    WorkflowPolicyDescriptor::new_async("regret", || Regret).on_workflow_failure(),
                )
                .step(StepDescriptor::new_async(CHARGE, async || declined()))
                .build()
                .unwrap();

            let (status, events) = travel(workflow.instance(Orders).create().unwrap());

            assert!(matches!(status, JourneyStatus::Failed { .. }));
            assert_eq!(
                kinds(&events),
                [
                    "journey_started",
                    "attempt_started",
                    "step_failed",
                    "step_given_up",
                    "journey_warning",
                    "hook_called",
                    "journey_failed"
                ]
            );
        }
    }
}
