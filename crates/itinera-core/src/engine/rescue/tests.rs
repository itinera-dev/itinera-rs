use std::sync::Arc;

use rstest::rstest;

use crate::engine::fixtures::{
    CHARGE, Seen, attempting, crashed, data_of, declined, failing_on, instance, kinds, seen,
    travel, travel_before, travel_workflow,
};
use crate::error::Error;
use crate::event::{Event, EventBody, EventKind};
use crate::journey::{
    Abort, Contributor, FailureCause, FailureRequester, JourneyStatus, MissingData,
};
use crate::policy::Requested;
use crate::step::{
    OptionalInput, Outcome, Reason, RescueFactory, RescueNeeds, RescueOutcome, RescueResolved,
    RescueStep, StepDescriptor, StepName,
};
use crate::workflow::fixtures::{Orders, orders};
use crate::workflow::{InputAdapter, InputAdapterDescriptor, Supplied};

const RESCUE: StepName = StepName::new("charge_rescue");
const ATTEMPT: OptionalInput<i64> = OptionalInput::new("attempt");

type Ends = fn() -> Result<Outcome, Error>;

/// The charge step, which ends each attempt as the next of `ends` says, with a rescue step that
/// succeeds.
fn rescued(ends: &'static [Ends]) -> StepDescriptor<Orders> {
    attempting(ends).rescue(|| Ok(RescueOutcome::success()))
}

/// Runs a journey of a workflow holding only the charge step.
fn travel_charge(charge: StepDescriptor<Orders>) -> (JourneyStatus, Vec<Event>) {
    travel_workflow(orders().step(charge))
}

/// The step a `journey_failed` event names, with the cause of the journey's failure.
fn failed_step(event: &Event) -> Option<(StepName, FailureCause)> {
    match &event.body {
        EventBody::JourneyFailed { step, failure } => Some((*step, failure.cause())),
        _ => None,
    }
}

#[rstest]
#[case::a_failure(rescued(&[declined]), EventKind::StepFailed)]
#[case::an_abnormal_termination_the_step_would_retry(
    rescued(&[crashed]),
    EventKind::StepAbnormalTermination
)]
#[case::an_abnormal_termination_the_step_does_not_retry(
    rescued(&[crashed]).no_retry_on_abnormal_termination(),
    EventKind::StepAbnormalTermination
)]
fn a_failed_first_attempt_is_rescued_then_the_step_is_attempted_again(
    #[case] charge: StepDescriptor<Orders>,
    #[case] fact: EventKind,
) {
    let (status, events) = travel_charge(charge);

    assert_eq!(data_of(&status), [("attempt".to_string(), 2)]);
    assert_eq!(
        kinds(&events),
        [
            EventKind::JourneyStarted,
            EventKind::AttemptStarted,
            fact,
            EventKind::StepRescuing,
            EventKind::AttemptStarted,
            EventKind::StepSucceeded,
            EventKind::StepRescued,
            EventKind::AttemptStarted,
            EventKind::StepSucceeded,
            EventKind::ContributionCommitted,
            EventKind::JourneySucceeded
        ]
    );
}

#[rstest]
#[case::a_failure(&[declined as Ends, declined], EventKind::StepFailed, FailureCause::Failure)]
#[case::an_abnormal_termination(
    &[declined as Ends, crashed],
    EventKind::StepAbnormalTermination,
    FailureCause::AbnormalTermination
)]
fn a_step_that_fails_after_its_rescue_is_given_up_without_a_second_rescue(
    #[case] ends: &'static [Ends],
    #[case] fact: EventKind,
    #[case] cause: FailureCause,
) {
    let (_, events) = travel_charge(rescued(ends));

    assert_eq!(
        kinds(&events),
        [
            EventKind::JourneyStarted,
            EventKind::AttemptStarted,
            EventKind::StepFailed,
            EventKind::StepRescuing,
            EventKind::AttemptStarted,
            EventKind::StepSucceeded,
            EventKind::StepRescued,
            EventKind::AttemptStarted,
            fact,
            EventKind::StepGivenUp,
            EventKind::JourneyFailed
        ]
    );
    assert_eq!(
        events.last().and_then(failed_step),
        Some((StepName::new("charge"), cause))
    );
}

fn refresh_failing() -> Result<RescueOutcome, Error> {
    Ok(RescueOutcome::failure("no-refresh"))
}

fn refresh_crashing() -> Result<RescueOutcome, Error> {
    Err(Error::msg("refresh down"))
}

#[rstest]
#[case::a_failure(refresh_failing, EventKind::StepFailed, FailureCause::Failure)]
#[case::an_abnormal_termination(
    refresh_crashing,
    EventKind::StepAbnormalTermination,
    FailureCause::AbnormalTermination
)]
fn a_rescue_step_that_does_not_succeed_fails_the_journey_and_the_step_is_not_attempted_again(
    #[case] refresh: fn() -> Result<RescueOutcome, Error>,
    #[case] fact: EventKind,
    #[case] cause: FailureCause,
) {
    let charge = attempting(&[declined]).rescue(refresh);

    let (_, events) = travel_charge(charge);

    assert_eq!(
        kinds(&events),
        [
            EventKind::JourneyStarted,
            EventKind::AttemptStarted,
            EventKind::StepFailed,
            EventKind::StepRescuing,
            EventKind::AttemptStarted,
            fact,
            EventKind::StepGivenUp,
            EventKind::JourneyFailed
        ]
    );
    assert_eq!(events.last().and_then(failed_step), Some((RESCUE, cause)));
}

/// A rescue step that records what the failed attempt left, contributes a token, and succeeds.
struct Refresh {
    seen: Seen,
}

struct Refreshing<'a> {
    attempt: Option<i64>,
    reason: Option<Reason>,
    error: Option<Error>,
    contributor: &'a mut Contributor,
    seen: &'a Seen,
}

impl RescueFactory for Refresh {
    type Step<'a> = Refreshing<'a>;

    fn needs(&self) -> RescueNeeds {
        RescueNeeds::new()
            .from_rescued_step(&ATTEMPT)
            .optional_reason()
            .optional_error()
            .contributor()
    }

    fn build<'a>(&'a self, got: &mut RescueResolved<'a>) -> Result<Refreshing<'a>, Error> {
        Ok(Refreshing {
            attempt: got.from_rescued_step(&ATTEMPT)?,
            reason: got.optional_reason()?,
            error: got.optional_error()?,
            contributor: got.contributor()?,
            seen: &self.seen,
        })
    }
}

impl RescueStep for Refreshing<'_> {
    fn run(self) -> Result<RescueOutcome, Error> {
        let reason = self.reason.as_ref().map(Reason::code);
        let error = self.error.as_ref().map(ToString::to_string);
        let saw = format!("{:?}, {reason:?}, {error:?}", self.attempt);
        self.seen.lock().unwrap().push(saw);
        self.contributor.contribute("token", 7_i64);
        Ok(RescueOutcome::success())
    }
}

#[rstest]
#[case::a_failure(&[declined as Ends], r#"Some(1), Some("declined"), None"#)]
#[case::an_abnormal_termination(&[crashed as Ends], r#"None, None, Some("the gateway crashed")"#)]
fn the_rescue_step_receives_what_the_failed_attempt_left_and_its_contributions_are_committed(
    #[case] ends: &'static [Ends],
    #[case] received: &str,
) {
    let saw = Seen::default();
    let refresh = Refresh {
        seen: Arc::clone(&saw),
    };
    let charge = attempting(ends).rescue(refresh);

    let (status, _) = travel_charge(charge);

    assert_eq!(seen(&saw), [received]);
    assert_eq!(
        data_of(&status),
        [("attempt".to_string(), 2), ("token".to_string(), 7)]
    );
}

/// A rescue step that requires the failure's reason, and succeeds.
struct NeedingReason;

impl RescueFactory for NeedingReason {
    type Step<'a> = Reason;

    fn needs(&self) -> RescueNeeds {
        RescueNeeds::new().reason()
    }

    fn build<'a>(&'a self, got: &mut RescueResolved<'a>) -> Result<Reason, Error> {
        got.reason()
    }
}

impl RescueStep for Reason {
    fn run(self) -> Result<RescueOutcome, Error> {
        Ok(RescueOutcome::success())
    }
}

fn aborted_at(event: &Event) -> Option<StepName> {
    match &event.body {
        EventBody::JourneyAborted { abort } => abort.step(),
        _ => None,
    }
}

#[test]
fn a_required_reason_an_abnormal_termination_did_not_leave_aborts_naming_the_rescue_step() {
    let charge = attempting(&[crashed]).rescue(NeedingReason);

    let (status, events) = travel_charge(charge);

    assert!(matches!(
        status,
        JourneyStatus::Aborted(Abort::RequiredDataMissing(MissingData::Reason {
            requester: FailureRequester::RescueStep
        }))
    ));
    assert_eq!(events.last().and_then(aborted_at), Some(RESCUE));
}

#[rstest]
#[case::on_step_rescuing("step_rescuing")]
#[case::on_step_rescued("step_rescued")]
fn a_reporter_failing_on_a_rescue_decision_aborts_the_journey_naming_the_rescued_step(
    #[case] kind: &'static str,
) {
    let workflow = orders().step(rescued(&[declined]));

    let (status, events) =
        travel_before(instance(workflow).create().unwrap(), Some(failing_on(kind)));

    assert!(matches!(
        status,
        JourneyStatus::Aborted(Abort::ReporterFailed(_))
    ));
    assert_eq!(events.last().and_then(aborted_at), Some(CHARGE));
}

#[cfg(feature = "async")]
mod asynchronous {
    use super::*;
    use crate::engine::fixtures::{async_orders, travel};

    #[test]
    fn an_asynchronous_rescue_step_is_awaited_before_the_step_is_attempted_again() {
        let charge = StepDescriptor::new_async(CHARGE, async || declined())
            .rescue_async(async || Ok(RescueOutcome::success()));
        let workflow = async_orders().step(charge).build().unwrap();

        let (_, events) = travel(workflow.instance(Orders).create().unwrap());

        assert_eq!(
            kinds(&events),
            [
                EventKind::JourneyStarted,
                EventKind::AttemptStarted,
                EventKind::StepFailed,
                EventKind::StepRescuing,
                EventKind::AttemptStarted,
                EventKind::StepSucceeded,
                EventKind::StepRescued,
                EventKind::AttemptStarted,
                EventKind::StepFailed,
                EventKind::StepGivenUp,
                EventKind::JourneyFailed
            ]
        );
    }
}

const TOKEN: OptionalInput<i64> = OptionalInput::new("token");

/// A rescue step that records the token it is built with, and succeeds.
struct ReadingToken {
    seen: Seen,
}

struct ReadToken<'a> {
    token: Option<i64>,
    seen: &'a Seen,
}

impl RescueFactory for ReadingToken {
    type Step<'a> = ReadToken<'a>;

    fn needs(&self) -> RescueNeeds {
        RescueNeeds::new().input(&TOKEN)
    }

    fn build<'a>(&'a self, got: &mut RescueResolved<'a>) -> Result<ReadToken<'a>, Error> {
        Ok(ReadToken {
            token: got.input(&TOKEN)?,
            seen: &self.seen,
        })
    }
}

impl RescueStep for ReadToken<'_> {
    fn run(self) -> Result<RescueOutcome, Error> {
        self.seen.lock().unwrap().push(format!("{:?}", self.token));
        Ok(RescueOutcome::success())
    }
}

/// Supplies a token of 7, and nothing else.
fn credentials(_: &Orders, got: Requested<'_, Orders, InputAdapter>) -> Result<Supplied, Error> {
    Ok(if got.key() == "token" {
        Supplied::value(7_i64)
    } else {
        Supplied::FromDataBag
    })
}

#[rstest]
#[case::attached_to_its_name(RESCUE, "Some(7)")]
#[case::attached_to_the_step_it_rescues(CHARGE, "Some(1)")]
fn a_rescue_steps_inputs_come_only_from_the_adapter_attached_to_its_name_then_the_data_bag(
    #[case] adapted: StepName,
    #[case] received: &str,
) {
    let saw = Seen::default();
    let refresh = ReadingToken {
        seen: Arc::clone(&saw),
    };
    let workflow = orders()
        .step(attempting(&[declined]).rescue(refresh))
        .input_adapter(InputAdapterDescriptor::new(
            "credentials",
            adapted,
            credentials,
        ));

    travel(instance(workflow).data("token", 1_i64).create().unwrap());

    assert_eq!(seen(&saw), [received]);
}
