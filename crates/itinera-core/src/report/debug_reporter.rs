//! A ready-made reporter that prints every event's `Debug`.

use std::io::{self, Write};

use crate::error::Error;
use crate::event::Event;
use crate::report::Reporter;

/// A reporter that prints every event of a journey on a line of its own, to standard output, as
/// the event's `Debug`: its sequence number, timestamp, journey ID, workflow and body.
///
/// It suits trying a workflow out, debugging one, and watching a test run. A reporter that prints
/// something else, or sends the events elsewhere, implements [`Reporter`] itself. A line standard
/// output cannot take is dropped, so printing never fails a journey.
///
/// # Examples
///
/// ```
/// use itinera::executor::LocalExecutor;
/// use itinera::report::DebugReporter;
/// use itinera::step::{Outcome, StepDescriptor, step_name};
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Orders;
///
/// let orders = WorkflowDescriptor::builder("orders")
///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
///     .reporter::<DebugReporter>()
///     .build()?;
/// let instance = orders.instance(Orders).create()?;
/// LocalExecutor::new().run(instance)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct DebugReporter;

impl Reporter for DebugReporter {
    fn report(&mut self, event: &Event) -> Result<(), Error> {
        // A line standard output cannot take is dropped: a reporter never fails a journey for
        // that.
        let _ = print(event, &mut io::stdout().lock());
        Ok(())
    }
}

/// Writes the event's `Debug` and a new line.
fn print(event: &Event, out: &mut impl Write) -> io::Result<()> {
    writeln!(out, "{event:?}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::fixtures::started;

    #[test]
    fn a_debug_reporter_prints_each_event_as_its_debug_on_a_line_of_its_own() {
        let event = started();
        let mut printed = Vec::new();
        print(&event, &mut printed).unwrap();
        assert_eq!(String::from_utf8(printed).unwrap(), format!("{event:?}\n"));
    }
}
