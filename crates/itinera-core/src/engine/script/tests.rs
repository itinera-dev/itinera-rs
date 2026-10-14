use super::*;
#[cfg(feature = "async")]
use crate::engine::fixtures::{CHARGE, async_orders};
use crate::engine::fixtures::{
    Read, charge, charged, failing_on, instance, kinds, reads, scripted,
};
use crate::event::{EventBody, Level};
use crate::instance::{Instance, InstanceBuilder, Scripted, WorkflowInstance};
use crate::journey::{
    Abort, Contributor, DataBag, JourneyStatus, MissingData, Read as JourneyRead, ScriptedRun,
    StatusKind,
};
#[cfg(feature = "async")]
use crate::mode::Asynchronous;
use crate::report::RecordingReporter;
#[cfg(feature = "async")]
use crate::step::{AsyncRescueFactory, AsyncRescueStep};
use crate::step::{
    OptionalInput, Outcome, RescueFactory, RescueNeeds, RescueOutcome, RescueResolved, RescueStep,
    Resolved, StepDescriptor, StepFactory, StepName, StepNeeds, StepReporter,
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

fn scripting(builder: InstanceBuilder<Orders>) -> Scripted<Instance<Orders>> {
    builder.create().unwrap().scripted()
}

fn data(run: &ScriptedRun) -> Vec<&str> {
    run.result.status.data().map(keys).unwrap_or_default()
}

fn keys(data: &DataBag) -> Vec<&str> {
    data.keys().collect()
}

#[test]
fn a_scripted_attempt_is_played_instead_of_running_its_step() {
    let run = scripting(handled())
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
    let run = scripting(instance(charged(&read)))
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
    let run = scripting(instance(orders().step(unbuildable)))
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
    let run = scripting(handled())
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
    scripting(handled().reporter(recording.clone()))
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
    let run = scripting(handled().reporter(failing_on("step_info")))
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
    let run = scripting(instance(retried).data("amount", 42_i64))
        .step("charge")
        .will_fail("timeout")
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
    let run = scripting(charge_with_amount(&read)).run().unwrap();

    assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
    assert_eq!(reads(&read), [(42, None)]);
}

#[test]
fn every_attempt_handed_to_its_steps_construction_is_recorded_scripted_or_run_for_real() {
    let read = Read::default();
    let retried = orders().step(charge(&read).retry_budget(1));
    let run = scripting(instance(retried).data("amount", 42_i64))
        .step("charge")
        .will_fail("timeout")
        .on_attempt(2)
        .will_run_for_real()
        .run()
        .unwrap();

    let charge = run.step("charge");
    assert_eq!(charge.attempts().len(), 2);
    assert_eq!(
        charge
            .attempt(2)
            .map(|attempt| attempt.input::<i64>("amount")),
        Some(JourneyRead::Present(42))
    );
}

#[test]
fn an_attempt_reads_each_input_its_step_was_built_with_and_absent_for_any_other() {
    let read = Read::default();
    let run = scripting(charge_with_amount(&read))
        .step("charge")
        .will_succeed()
        .run()
        .unwrap();

    let first = run.step("charge").attempt(1).unwrap();
    assert_eq!(first.input::<i64>("amount"), JourneyRead::Present(42));
    assert_eq!(first.input::<i64>("discount"), JourneyRead::Absent);
}

#[test]
fn an_attempt_whose_step_cannot_be_built_is_still_recorded() {
    let unbuildable = StepDescriptor::new(StepName::new("charge"), Unbuildable);
    let run = scripting(instance(orders().step(unbuildable)))
        .step("charge")
        .will_succeed()
        .run()
        .unwrap();

    assert_eq!(run.step("charge").attempts().len(), 1);
}

#[test]
fn an_attempt_that_ends_before_its_step_is_built_is_not_recorded() {
    let read = Read::default();
    let run = scripting(instance(charged(&read)))
        .step("charge")
        .will_succeed()
        .run()
        .unwrap();

    assert!(run.step("charge").attempts().is_empty());
}

const ATTEMPT: OptionalInput<i64> = OptionalInput::new("attempt");

/// A rescue step that asks for what the failed attempt contributed under `attempt`.
struct Refresh;

impl RescueFactory for Refresh {
    type Step<'a> = Refreshing;

    fn needs(&self) -> RescueNeeds {
        RescueNeeds::new().optional_from_rescued_step(&ATTEMPT)
    }

    fn build<'a>(&'a self, got: &mut RescueResolved<'a>) -> Result<Refreshing, Error> {
        got.optional_from_rescued_step(&ATTEMPT)?;
        Ok(Refreshing)
    }
}

struct Refreshing;

impl RescueStep for Refreshing {
    fn run(self) -> Result<RescueOutcome, Error> {
        Ok(RescueOutcome::success())
    }
}

#[test]
fn a_rescue_steps_attempt_records_what_it_requested_from_the_failed_attempt() {
    let rescued = orders().step(scripted(contributes_for_real).rescue(Refresh));
    let run = scripting(instance(rescued))
        .step("charge")
        .will_contribute("attempt", 1_i64)
        .will_fail("declined")
        .on_attempt(2)
        .will_succeed()
        .and_step("charge_rescue")
        .will_succeed()
        .run()
        .unwrap();

    let rescue = run.step("charge_rescue").attempt(1).unwrap();
    assert_eq!(rescue.input::<i64>("attempt"), JourneyRead::Present(1));
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
