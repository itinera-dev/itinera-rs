use super::*;
#[cfg(feature = "async")]
use crate::engine::fixtures::{CHARGE, async_orders};
use crate::engine::fixtures::{
    Read, charge, charged, failing_on, instance, kinds, reads, scripted,
};
use crate::event::{EventBody, Level};
use crate::instance::{InstanceBuilder, WorkflowInstance};
use crate::journey::{
    Abort, Contributor, DataBag, JourneyStatus, MissingData, ScriptedRun, StatusKind,
};
#[cfg(feature = "async")]
use crate::mode::Asynchronous;
use crate::report::RecordingReporter;
#[cfg(feature = "async")]
use crate::step::{
    AsyncRescueFactory, AsyncRescueStep, RescueNeeds, RescueOutcome, RescueResolved,
};
use crate::step::{
    Outcome, Resolved, StepDescriptor, StepFactory, StepName, StepNeeds, StepReporter,
};
use crate::value::AnyValue;
use crate::workflow::fixtures::{Orders, orders};

fn charge_with_amount(read: &Read) -> InstanceBuilder<Orders> {
    instance(charged(read)).data("amount", 42_i64)
}

/// What the charge step does when it runs for real: it contributes `ran`.
fn contributes_for_real(
    contributor: &mut Contributor<'_>,
    _: &mut StepReporter<'_>,
) -> Result<Outcome, Error> {
    contributor.contribute("ran", true);
    Ok(Outcome::success())
}

/// The instance of the orders workflow whose charge step declares a contributor and a reporter.
fn handled() -> InstanceBuilder<Orders> {
    instance(orders().step(scripted(contributes_for_real)))
}

fn data(run: &ScriptedRun) -> Vec<&str> {
    run.result.status.data().map(keys).unwrap_or_default()
}

fn keys(data: &DataBag) -> Vec<&str> {
    data.keys().collect()
}

#[test]
fn a_scripted_attempt_is_played_instead_of_running_its_step() {
    let run = handled()
        .create()
        .unwrap()
        .scripted()
        .step("charge")
        .will_contribute("charged", 40_i64)
        .will_succeed()
        .run()
        .unwrap();

    assert_eq!(data(&run), ["charged"]);
}

#[test]
fn a_scripted_step_is_still_built_so_a_missing_input_aborts_the_journey() {
    let read = Read::default();
    let run = instance(charged(&read))
        .create()
        .unwrap()
        .scripted()
        .step("charge")
        .will_succeed()
        .run()
        .unwrap();

    assert!(matches!(
        run.result.status,
        JourneyStatus::Aborted(Abort::RequiredDataMissing(MissingData::Key { .. }))
    ));
}

struct Unbuildable;

impl StepFactory for Unbuildable {
    type Step<'a> = &'a fn() -> Result<Outcome, Error>;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new()
    }

    fn build<'a>(&'a self, _: &mut Resolved<'a>) -> Result<Self::Step<'a>, Error> {
        Err(Error::msg("no ledger"))
    }
}

#[test]
fn a_scripted_step_whose_constructor_fails_aborts_the_journey() {
    let unbuildable = StepDescriptor::new(StepName::new("charge"), Unbuildable);
    let run = instance(orders().step(unbuildable))
        .create()
        .unwrap()
        .scripted()
        .step("charge")
        .will_succeed()
        .run()
        .unwrap();

    let JourneyStatus::Aborted(Abort::StepCouldNotBeBuilt(error)) = run.result.status else {
        panic!("the journey was not aborted because the step could not be built");
    };
    assert_eq!(error.to_string(), "no ledger");
}

#[test]
fn a_failed_scripted_attempts_contributions_are_never_committed() {
    let run = handled()
        .create()
        .unwrap()
        .scripted()
        .step("charge")
        .will_contribute("charged", 40_i64)
        .will_fail("declined")
        .run()
        .unwrap();

    assert!(data(&run).is_empty());
}

#[test]
fn a_scripted_attempt_emits_and_sets_global_metadata_as_its_step_would_in_order() {
    let recording = RecordingReporter::default();
    handled()
        .reporter(recording.clone())
        .create()
        .unwrap()
        .scripted()
        .step("charge")
        .will_emit_with(Level::Warning, "slow", 300_i64)
        .will_set_global_metadata("receipt", "R-1".to_owned())
        .will_emit(Level::Info, "charged")
        .will_succeed()
        .run()
        .unwrap();

    let events = recording.events();
    assert_eq!(
        kinds(&events),
        [
            "journey_started",
            "attempt_started",
            "step_warning",
            "global_metadata_set",
            "step_info",
            "step_succeeded",
            "journey_succeeded"
        ]
    );
    assert!(matches!(
        events.get(2).map(|event| &event.body),
        Some(EventBody::StepWarning { step, message, .. })
            if step.step == StepName::new("charge") && message == "slow"
    ));
    let metadata = recording.global_metadata();
    let receipt = metadata.get("receipt").and_then(AnyValue::downcast_ref);
    assert_eq!(receipt, Some(&"R-1".to_owned()));
}

#[test]
fn a_reporter_failing_on_a_scripted_emit_aborts_the_journey() {
    let run = handled()
        .reporter(failing_on("step_info"))
        .create()
        .unwrap()
        .scripted()
        .step("charge")
        .will_emit(Level::Info, "charging")
        .will_succeed()
        .run()
        .unwrap();

    assert!(matches!(
        run.result.status,
        JourneyStatus::Aborted(Abort::ReporterFailed(_))
    ));
}

#[test]
fn an_attempt_scripted_to_run_for_real_runs_its_step() {
    let read = Read::default();
    let retried = orders().step(charge(&read).retry_budget(1));
    let run = instance(retried)
        .data("amount", 42_i64)
        .create()
        .unwrap()
        .scripted()
        .step("charge")
        .will_fail_retriably("timeout")
        .on_attempt(2)
        .will_run_for_real()
        .run()
        .unwrap();

    assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
    assert_eq!(reads(&read), [(42, None)]);
}

#[test]
fn a_step_the_script_does_not_name_runs_for_real() {
    let read = Read::default();
    let run = charge_with_amount(&read)
        .create()
        .unwrap()
        .scripted()
        .run()
        .unwrap();

    assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
    assert_eq!(reads(&read), [(42, None)]);
}

#[cfg(feature = "async")]
struct UnbuildableRescue;

#[cfg(feature = "async")]
impl AsyncRescueFactory for UnbuildableRescue {
    type Step<'a> = Unbuilt;

    fn needs(&self) -> RescueNeeds {
        RescueNeeds::new()
    }

    fn build<'a>(&'a self, _: &mut RescueResolved<'a, Asynchronous>) -> Result<Unbuilt, Error> {
        Err(Error::msg("no token service"))
    }
}

#[cfg(feature = "async")]
struct Unbuilt;

#[cfg(feature = "async")]
impl AsyncRescueStep for Unbuilt {
    async fn run(self) -> Result<RescueOutcome, Error> {
        Ok(RescueOutcome::success())
    }
}

#[cfg(feature = "async")]
#[test]
fn a_scripted_asynchronous_rescue_step_is_still_built_so_a_failing_constructor_aborts_the_journey()
{
    let charge = StepDescriptor::new_async(CHARGE, async || Ok(Outcome::success()))
        .rescue_async(UnbuildableRescue);
    let scripted = async_orders()
        .step(charge)
        .build()
        .unwrap()
        .instance(Orders)
        .create()
        .unwrap()
        .scripted()
        .step("charge")
        .will_fail("declined")
        .on_attempt(2)
        .will_succeed()
        .and_step("charge_rescue")
        .will_succeed();

    let run = futures::executor::block_on(scripted.run()).unwrap();

    let JourneyStatus::Aborted(Abort::StepCouldNotBeBuilt(error)) = run.result.status else {
        panic!("the journey was not aborted because the rescue step could not be built");
    };
    assert_eq!(error.to_string(), "no token service");
}
