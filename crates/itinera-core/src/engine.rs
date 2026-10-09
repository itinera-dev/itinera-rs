//! The engine both executors share: it runs one journey, emitting its events, and makes its
//! result.

use std::future::Future;
use std::pin::Pin;

use crate::error::{Error, Interrupted};
use crate::event::{
    self, Event, EventBody, GiveUpCause, JourneyAbort, JourneyFailure, RequestSource, Source,
};
use crate::instance::{Committed, Resolution, WorkflowInstance};
use crate::journey::{
    Abort, Contribution, Contributions, Contributor, Failure, JourneyResult, JourneyStatus,
    LastFailure, MissingData, Requester,
};
use crate::report::Dispatcher;
use crate::step::{
    Got, InputNeed, Outcome, OutcomeKind, Reporting, Requirement, StepAttempt, StepDescriptor,
    StepName,
};
use crate::value::AnyValue;
use crate::workflow::WorkflowDescriptor;

#[cfg(feature = "async")]
mod asynchronous;
mod emitter;
mod guard;

#[cfg(feature = "async")]
pub(crate) use asynchronous::Awaited;
pub(crate) use emitter::Clock;
pub(crate) use guard::Failures;

use emitter::Emitter;

/// Delivers the journey's events through its dispatcher, whichever kind it is.
pub(crate) trait Delivery: Send {
    fn deliver(&mut self, event: &Event) -> impl Future<Output = Result<(), Error>> + Send;
}

/// A synchronous dispatcher, called at once.
#[derive(derive_more::From)]
pub(crate) struct Inline<D> {
    dispatcher: D,
}

impl<D: Dispatcher> Delivery for Inline<D> {
    fn deliver(&mut self, event: &Event) -> impl Future<Output = Result<(), Error>> + Send {
        std::future::ready(self.dispatcher.dispatch(event))
    }
}

/// An event a step or hook emitted, delivered once awaited.
pub(crate) type Emitted<'a> = Pin<Box<dyn Future<Output = Result<(), Interrupted>> + Send + 'a>>;

/// The journey, as the handles of steps and hooks emit through it.
pub(crate) trait Emitting: Send {
    /// Delivers an event a step or hook emitted, unless a reporter failed while it ran.
    fn relay(&mut self, body: EventBody) -> Emitted<'_>;
}

/// Runs one journey of the instance, delivering its events through the dispatcher, whose
/// reporters were guarded by `failures`.
pub(crate) async fn run<I: WorkflowInstance>(
    mut instance: I,
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
    let status = match journey.travel(&mut instance, &descriptor).await {
        Ok(()) => JourneyStatus::Succeeded {
            data: instance.into_data_bag(),
        },
        Err(End::Failed(failure)) => JourneyStatus::Failed {
            failure,
            data: instance.into_data_bag(),
        },
        Err(End::Aborted(aborted)) => journey.abort(*aborted).await,
    };
    JourneyResult { journey_id, status }
}

/// How a journey ends before its last step, instead of succeeding.
#[derive(derive_more::From)]
enum End {
    Failed(Failure),
    Aborted(Box<Aborted>),
}

/// An abort, for the result and as `journey_aborted` reports it.
struct Aborted {
    abort: Abort,
    reported: JourneyAbort,
}

impl End {
    fn aborted(abort: Abort, reported: JourneyAbort) -> Self {
        Self::Aborted(Box::new(Aborted { abort, reported }))
    }
}

/// One journey while it runs.
struct Journey<D> {
    emitter: Emitter,
    delivery: D,
    failures: Failures,
    /// The step being run, if any.
    step: Option<StepName>,
    /// The abort recorded when a reporter failed on an event a step emitted, while it was built
    /// or ran. It stands whatever the step does afterwards.
    interrupted: Option<Box<Aborted>>,
}

impl<D: Delivery> Journey<D> {
    /// Runs the journey's steps in order, until one ends it or none is left.
    ///
    /// It holds the instance only mutably across an await, so that the journey's future is
    /// `Send` whenever the instance is.
    async fn travel<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        descriptor: &WorkflowDescriptor<I::Workflow, I::Mode>,
    ) -> Result<(), End> {
        let initial_keys = instance.data_bag().keys().map(str::to_owned).collect();
        self.emit(EventBody::JourneyStarted { initial_keys })
            .await?;
        for step in descriptor.steps() {
            self.step = Some(step.name());
            self.attempt(instance, step).await?;
            self.step = None;
        }
        self.emit(EventBody::JourneySucceeded { decided_by: None })
            .await?;
        Ok(())
    }

    /// Builds and runs the one attempt a step has until retries exist, and acts on its outcome.
    async fn attempt<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        step: &StepDescriptor<I::Mode>,
    ) -> Result<(), End> {
        let attempt = StepAttempt::first(step.name());
        self.emit(EventBody::AttemptStarted {
            step: attempt.clone(),
        })
        .await?;
        let mut inputs = Vec::new();
        for input in step.needs().inputs() {
            let resolution = instance.data_for_step(step.name(), input.key());
            let value = self.resolve(&attempt, input, resolution).await?;
            inputs.push((input.key(), value));
        }
        let mut contributions = Contributions::default();
        let ran = self
            .build_and_run(step, &attempt, inputs, &mut contributions)
            .await;
        if let Some(aborted) = self.interrupted.take() {
            return Err(End::Aborted(aborted));
        }
        match ran? {
            Ok(outcome) => {
                self.conclude(instance, attempt, outcome.into(), contributions)
                    .await
            }
            Err(error) => self.terminate(attempt, error).await,
        }
    }

    /// Builds the step for its attempt, with the inputs resolved and the handles it declares, and
    /// runs it.
    async fn build_and_run<M>(
        &mut self,
        step: &StepDescriptor<M>,
        attempt: &StepAttempt,
        inputs: Vec<(&'static str, Option<AnyValue>)>,
        contributions: &mut Contributions,
    ) -> Result<Result<Outcome, Error>, End> {
        let needs = step.needs();
        let contributor = if needs.wants_contributor() {
            Some(Contributor::new(contributions))
        } else {
            None
        };
        let reporting = if needs.wants_reporter() {
            Some(Reporting::new(self, attempt.clone()))
        } else {
            None
        };
        match step.attempt(Got::new(inputs, contributor, reporting)) {
            Ok(running) => Ok(running.await),
            Err(error) => Err(could_not_build(step.name(), error)),
        }
    }

    /// Turns what the instance answered for one input into its events and its value, or the
    /// abort it causes.
    async fn resolve(
        &mut self,
        attempt: &StepAttempt,
        input: &InputNeed,
        resolution: Resolution,
    ) -> Result<Option<AnyValue>, End> {
        let step = attempt.step;
        let key = input.key();
        match resolution {
            Resolution::Supplied { adapter, value } => {
                self.emit(EventBody::InputAdapterSupplied {
                    step: attempt.clone(),
                    key: key.to_owned(),
                    adapter,
                })
                .await?;
                if input.accepts(&value) {
                    Ok(Some(value))
                } else {
                    Err(wrong_type(
                        key,
                        Requester::Adapter { adapter },
                        event::Requester::Adapter { adapter, step },
                    ))
                }
            }
            Resolution::AdapterFailed { adapter, error } => {
                self.emit(EventBody::InputAdapterFailed {
                    step: attempt.clone(),
                    key: key.to_owned(),
                    adapter,
                })
                .await?;
                Err(could_not_build(step, error))
            }
            Resolution::InDataBag(value) if input.accepts(&value) => Ok(Some(value)),
            Resolution::InDataBag(_) => Err(wrong_type(
                key,
                Requester::Step,
                event::Requester::Step { step },
            )),
            Resolution::Absent => match input.requirement() {
                Requirement::Required => Err(missing(step, key)),
                Requirement::Optional => self.absent(attempt, key).await,
            },
        }
    }

    /// Reports that an optional input has no value, with which the step is built.
    async fn absent(&mut self, attempt: &StepAttempt, key: &str) -> Result<Option<AnyValue>, End> {
        self.emit(EventBody::OptionalInputAbsent {
            key: key.to_owned(),
            requester: RequestSource::Step(attempt.clone()),
        })
        .await?;
        Ok(None)
    }

    /// Acts on the outcome a step reported, and on its contributions. Until retries exist, a
    /// failure is the step's last.
    async fn conclude<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        attempt: StepAttempt,
        outcome: OutcomeKind,
        contributions: Contributions,
    ) -> Result<(), End> {
        match outcome {
            OutcomeKind::Success => {
                self.emit(EventBody::StepSucceeded {
                    step: attempt.clone(),
                })
                .await?;
                self.commit(instance, &attempt, contributions).await
            }
            OutcomeKind::Skipped(reason) => {
                self.emit(EventBody::StepSkipped {
                    step: attempt.clone(),
                    reason,
                })
                .await?;
                self.emit(EventBody::ContributionsDiscarded { step: attempt })
                    .await?;
                Ok(())
            }
            OutcomeKind::Failure(reason) => {
                self.emit(EventBody::StepFailed {
                    step: attempt.clone(),
                    retriable: false,
                    reason: reason.clone(),
                })
                .await?;
                self.give_up(
                    attempt,
                    GiveUpCause::Failure,
                    JourneyFailure::Failure(reason.clone()),
                    Failure::Failure(reason),
                )
                .await
            }
            OutcomeKind::RetriableFailure(reason) => {
                self.emit(EventBody::StepFailed {
                    step: attempt.clone(),
                    retriable: true,
                    reason: reason.clone(),
                })
                .await?;
                self.give_up(
                    attempt,
                    GiveUpCause::RetriesExhausted,
                    JourneyFailure::RetriesExhausted(LastFailure::Reason(reason.clone())),
                    Failure::RetriesExhausted(LastFailure::Reason(reason)),
                )
                .await
            }
        }
    }

    /// Commits a successful attempt's contributions to the data bag, in order.
    async fn commit<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        attempt: &StepAttempt,
        contributions: Contributions,
    ) -> Result<(), End> {
        for Contribution { key, value } in contributions {
            let committed = instance.commit(key.clone(), value);
            let source = Source::Step(attempt.clone());
            self.emit(EventBody::ContributionCommitted {
                key: key.clone(),
                source: source.clone(),
            })
            .await?;
            if committed == Committed::Overwritten {
                self.emit(EventBody::DataOverwritten { key, source })
                    .await?;
            }
        }
        Ok(())
    }

    /// Acts on an error that escaped a running step: an abnormal termination, which is not
    /// retried by default.
    async fn terminate(&mut self, attempt: StepAttempt, error: Error) -> Result<(), End> {
        let message = error.to_string();
        self.emit(EventBody::StepAbnormalTermination {
            step: attempt.clone(),
            message: message.clone(),
        })
        .await?;
        self.give_up(
            attempt,
            GiveUpCause::AbnormalTermination,
            JourneyFailure::AbnormalTermination(message),
            Failure::AbnormalTermination(error),
        )
        .await
    }

    /// Decides by default that the step will not be attempted again, and that the journey fails.
    async fn give_up(
        &mut self,
        attempt: StepAttempt,
        cause: GiveUpCause,
        reported: JourneyFailure,
        failure: Failure,
    ) -> Result<(), End> {
        let step = attempt.step;
        self.emit(EventBody::StepGivenUp {
            step: attempt,
            cause,
        })
        .await?;
        self.emit(EventBody::JourneyFailed {
            step,
            failure: reported,
        })
        .await?;
        Err(End::Failed(failure))
    }

    /// Emits one event, failing with the error of the reporter that failed on it, or else of
    /// the dispatcher.
    async fn emit(&mut self, body: EventBody) -> Result<(), Box<Aborted>> {
        let event = self.emitter.next(body);
        let delivered = self.delivery.deliver(&event).await;
        match self.failures.take().map_or(delivered, Err) {
            Ok(()) => Ok(()),
            Err(error) => Err(reporter_failed(self.step, error)),
        }
    }

    /// Emits an event for a step or hook, unless a reporter failed while it ran. A failure now is
    /// recorded, and interrupts it.
    async fn relay_interruptibly(&mut self, body: EventBody) -> Result<(), Interrupted> {
        if self.interrupted.is_some() {
            return Err(Interrupted);
        }
        match self.emit(body).await {
            Ok(()) => Ok(()),
            Err(aborted) => {
                self.interrupted = Some(aborted);
                Err(Interrupted)
            }
        }
    }

    /// Ends the journey as aborted. A failure while `journey_aborted` itself is delivered is
    /// ignored.
    async fn abort(&mut self, Aborted { abort, reported }: Aborted) -> JourneyStatus {
        let event = self
            .emitter
            .next(EventBody::JourneyAborted { abort: reported });
        let _ignored = self.delivery.deliver(&event).await;
        JourneyStatus::Aborted(abort)
    }
}

impl<D: Delivery> Emitting for Journey<D> {
    fn relay(&mut self, body: EventBody) -> Emitted<'_> {
        Box::pin(self.relay_interruptibly(body))
    }
}

fn could_not_build(step: StepName, error: Error) -> End {
    let reported = JourneyAbort::StepCouldNotBeBuilt {
        step,
        error: error.to_string(),
    };
    End::aborted(Abort::StepCouldNotBeBuilt(error), reported)
}

fn missing(step: StepName, key: &str) -> End {
    End::aborted(
        Abort::RequiredDataMissing(MissingData::Key {
            key: key.to_owned(),
            requester: Requester::Step,
        }),
        JourneyAbort::RequiredDataMissing {
            missing: event::MissingData::Key {
                key: key.to_owned(),
                requester: event::Requester::Step { step },
            },
        },
    )
}

/// The abort for an input of the wrong type, naming the input adapter that supplied it, or else
/// the step, when the data bag held it.
fn wrong_type(key: &str, requester: Requester, reported: event::Requester) -> End {
    End::aborted(
        Abort::WrongType {
            key: key.to_owned(),
            requester,
        },
        JourneyAbort::WrongType {
            key: key.to_owned(),
            requester: reported,
        },
    )
}

fn reporter_failed(step: Option<StepName>, error: Error) -> Box<Aborted> {
    let reported = JourneyAbort::ReporterFailed {
        step,
        error: error.to_string(),
    };
    Box::new(Aborted {
        abort: Abort::ReporterFailed(error),
        reported,
    })
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
mod tests {
    use std::num::NonZeroU64;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use rstest::rstest;

    use super::*;
    use crate::event::{Event, Timestamp};
    use crate::instance::InstanceBuilder;
    use crate::journey::FailureCause;
    use crate::report::{DefaultDispatcher, Reporter};
    use crate::step::{
        Input, OptionalInput, Outcome, Reason, Resolved, Step, StepDescriptor, StepFactory,
        StepNeeds, StepReporter,
    };
    use crate::workflow::{
        AdapterName, InputAdapter, WorkflowBuilder, WorkflowDescriptor, WorkflowName,
    };

    struct Orders;

    /// A reporter that records every event it receives, and fails on one kind if told to.
    #[derive(Default)]
    struct Recording {
        events: Arc<Mutex<Vec<Event>>>,
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

    const CHARGE: StepName = StepName::new("charge");
    const SHIP: StepName = StepName::new("ship");
    const AMOUNT: Input<i64> = Input::new("amount");
    const DISCOUNT: OptionalInput<i64> = OptionalInput::new("discount");

    /// What the charge step read, one entry per attempt.
    type Read = Arc<Mutex<Vec<(i64, Option<i64>)>>>;

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

    fn scripted<S>(script: S) -> StepDescriptor
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

    fn failing_on(kind: &'static str) -> Recording {
        Recording {
            fails_on: Some(kind),
            ..Recording::default()
        }
    }

    fn noon() -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(1_700_000_000)
    }

    /// Runs the journey to its end, and returns its status and the events it emitted.
    fn travel<I: WorkflowInstance>(instance: I) -> (JourneyStatus, Vec<Event>) {
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
        let failures = Failures::default();
        let mut dispatcher = DefaultDispatcher::new();
        for reporter in [recording].into_iter().chain(after) {
            dispatcher.add(failures.guard(Box::new(reporter))).unwrap();
        }
        let result = finish(run(instance, Inline::from(dispatcher), failures, noon));
        let events = events.lock().unwrap().clone();
        (result.status, events)
    }

    fn orders() -> WorkflowBuilder<Orders> {
        WorkflowDescriptor::builder("orders")
    }

    /// The orders workflow with the charge step, which records what it read in `read`.
    fn charged(read: &Read) -> WorkflowBuilder<Orders> {
        let charge = Charge {
            read: Arc::clone(read),
        };
        orders().step(StepDescriptor::new(CHARGE, charge))
    }

    /// Runs a journey of the workflow, whose initial data holds an amount of 42.
    fn travel_with_amount(builder: WorkflowBuilder<Orders>) -> (JourneyStatus, Vec<Event>) {
        travel(instance(builder).data("amount", 42_i64).create().unwrap())
    }

    /// Runs a journey of the workflow, with no initial data.
    fn travel_workflow(builder: WorkflowBuilder<Orders>) -> (JourneyStatus, Vec<Event>) {
        travel(instance(builder).create().unwrap())
    }

    fn instance(builder: WorkflowBuilder<Orders>) -> InstanceBuilder<Orders> {
        builder.build().unwrap().instance(Orders)
    }

    fn succeed() -> Result<Outcome, Error> {
        Ok(Outcome::success())
    }

    fn kinds(events: &[Event]) -> Vec<&'static str> {
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
    fn pricing(_: &Orders, _: StepName, key: &str) -> Result<Option<AnyValue>, Error> {
        Ok((key == "amount").then(|| AnyValue::new(7_i64)))
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
        let workflow = charged(&read).input_adapter(InputAdapter::new("pricing", CHARGE, pricing));

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
        let failing = InputAdapter::new("pricing", CHARGE, |_: &Orders, _, _| {
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
        let wrong = InputAdapter::new("pricing", CHARGE, |_: &Orders, _, _| {
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

    fn declined() -> Result<Outcome, Error> {
        Ok(Outcome::failure(Reason::new("declined")))
    }

    fn timed_out() -> Result<Outcome, Error> {
        Ok(Outcome::retriable_failure(Reason::new("timeout")))
    }

    fn crashed() -> Result<Outcome, Error> {
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
    fn a_failed_attempt_is_the_steps_last_and_fails_the_journey(
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

    fn data_of(status: &JourneyStatus) -> Vec<(String, i64)> {
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
    }
}
