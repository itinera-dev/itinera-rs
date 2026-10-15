//! Asynchronous steps and hooks the macros declare, called directly by a test with the handles
//! the recorders lend, outside any journey.

use futures::executor::block_on;
use itinera::error::Error;
use itinera::journey::{Contributions, Contributor, Read};
use itinera::policy::{AsyncHookReporter, HookEventReporter, workflow_policy};
use itinera::step::{AsyncStepReporter, Outcome, Step, StepEventReporter, step};

#[derive(Step)]
struct Charge {
    #[input]
    price: i64,
}

#[step]
impl Charge {
    async fn run(
        self,
        contributor: &mut Contributor,
        mut reporter: AsyncStepReporter<'_>,
    ) -> Result<Outcome, Error> {
        reporter.info_with("charging", self.price).await?;
        contributor.contribute("charged", self.price);
        Self::succeed()
    }
}

#[derive(Default)]
struct Close;

#[workflow_policy(name = "close")]
impl Close {
    #[on_workflow_failure]
    async fn failed(&self, mut reporter: AsyncHookReporter<'_>) -> Result<(), Error> {
        reporter.warning("the order failed").await?;
        Ok(())
    }
}

#[test]
fn an_asynchronous_steps_run_called_with_recorded_handles_leaves_what_it_did_to_read() {
    let mut contributions = Contributions::default();
    let mut events = StepEventReporter::default();

    let outcome =
        block_on(Charge { price: 40 }.run(contributions.contributor(), events.async_reporter()))
            .unwrap();

    assert!(outcome.is_success());
    assert_eq!(contributions.read::<i64>("charged"), Read::Present(40));
    assert_eq!(events.emitted().first().unwrap().message(), "charging");
}

#[test]
fn an_asynchronous_hook_passes_on_a_failing_recorders_interruption() {
    let mut events = HookEventReporter::failing();

    let called = block_on(Close.failed(events.async_reporter()));

    assert!(called.is_err());
    assert!(events.emitted().is_empty());
}
