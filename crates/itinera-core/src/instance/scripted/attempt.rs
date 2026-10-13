//! Scripting one attempt of a step: what it does, then how it ends.

use std::marker::PhantomData;

use super::slot::Slot;
use super::{StepScripting, WorkflowInstance};
use crate::event::Level;
use crate::step::{Action, AttemptScript, Ending, Played, Reason};
use crate::value::{AnyValue, Value};

mod state;

pub use state::{Acting, Named, Opened};

/// The script of one or more attempts of a step, being written: what they do, in order, then
/// how they end, which returns to the step's script.
///
/// `S` is how far it got: [`Named`], [`Opened`] or [`Acting`].
///
/// # Examples
///
/// ```
/// use itinera::event::Level;
/// use itinera::instance::WorkflowInstance;
/// use itinera::journey::StatusKind;
/// use itinera::step::{Outcome, StepDescriptor, step_name};
/// use itinera::workflow::WorkflowDescriptor;
///
/// # struct Charge<'a> {
/// #     contributor: itinera::journey::Contributor<'a>,
/// #     reporter: itinera::step::StepReporter<'a>,
/// # }
/// # impl itinera::step::Step for Charge<'_> {
/// #     fn run(mut self) -> Result<Outcome, itinera::error::Error> {
/// #         self.reporter.info("charging")?;
/// #         self.contributor.contribute("charged", 40_i64);
/// #         Ok(Outcome::success())
/// #     }
/// # }
/// # struct ChargeFactory;
/// # impl itinera::step::StepFactory for ChargeFactory {
/// #     type Step<'a> = Charge<'a>;
/// #     fn needs(&self) -> itinera::step::StepNeeds {
/// #         itinera::step::StepNeeds::new().contributor().reporter()
/// #     }
/// #     fn build<'a>(
/// #         &'a self,
/// #         got: &mut itinera::step::Resolved<'a>,
/// #     ) -> Result<Charge<'a>, itinera::error::Error> {
/// #         let contributor = got.contributor()?;
/// #         let reporter = got.reporter()?;
/// #         Ok(Charge { contributor, reporter })
/// #     }
/// # }
/// // ChargeFactory builds a charge step that declares a contributor and a reporter.
///
/// struct Orders;
///
/// let orders = WorkflowDescriptor::builder("orders")
///     .step(StepDescriptor::new(step_name!("charge"), ChargeFactory))
///     .build()?;
/// let run = orders
///     .instance(Orders)
///     .create()?
///     .scripted()
///     .step("charge")
///     .will_emit(Level::Info, "charging")
///     .will_contribute("charged", 40_i64)
///     .will_succeed()
///     .run()?;
/// assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(derive_more::Debug)]
#[debug("AttemptScripting({slot:?}, {actions:?})")]
pub struct AttemptScripting<I: WorkflowInstance, M, S> {
    step: StepScripting<I, M>,
    slot: Slot,
    actions: Vec<Action>,
    state: PhantomData<fn() -> S>,
}

impl<I: WorkflowInstance, M, S> AttemptScripting<I, M, S> {
    pub(super) fn new(step: StepScripting<I, M>, slot: Slot) -> Self {
        Self {
            step,
            slot,
            actions: Vec::new(),
            state: PhantomData,
        }
    }

    /// The attempt contributes `value` under `key`, committed to the data bag only if it
    /// succeeds, as a step's contributions are.
    ///
    /// # Panics
    ///
    /// When the step declares no contributor, so the real step could not contribute.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// # struct Charge<'a> {
    /// #     contributor: itinera::journey::Contributor<'a>,
    /// #     reporter: itinera::step::StepReporter<'a>,
    /// # }
    /// # impl itinera::step::Step for Charge<'_> {
    /// #     fn run(mut self) -> Result<Outcome, itinera::error::Error> {
    /// #         self.reporter.info("charging")?;
    /// #         self.contributor.contribute("charged", 40_i64);
    /// #         Ok(Outcome::success())
    /// #     }
    /// # }
    /// # struct ChargeFactory;
    /// # impl itinera::step::StepFactory for ChargeFactory {
    /// #     type Step<'a> = Charge<'a>;
    /// #     fn needs(&self) -> itinera::step::StepNeeds {
    /// #         itinera::step::StepNeeds::new().contributor().reporter()
    /// #     }
    /// #     fn build<'a>(
    /// #         &'a self,
    /// #         got: &mut itinera::step::Resolved<'a>,
    /// #     ) -> Result<Charge<'a>, itinera::error::Error> {
    /// #         let contributor = got.contributor()?;
    /// #         let reporter = got.reporter()?;
    /// #         Ok(Charge { contributor, reporter })
    /// #     }
    /// # }
    /// // ChargeFactory builds a charge step that declares a contributor and a reporter.
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), ChargeFactory))
    ///     .build()?;
    /// let run = orders
    ///     .instance(Orders)
    ///     .create()?
    ///     .scripted()
    ///     .step("charge")
    ///     .will_contribute("charged", 40_i64)
    ///     .will_succeed()
    ///     .run()?;
    /// let data = run.result.status.data().unwrap();
    /// assert_eq!(data.required::<i64>("charged")?, 40);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn will_contribute(
        self,
        key: impl Into<String>,
        value: impl Value,
    ) -> AttemptScripting<I, M, Acting> {
        self.acting(Action::Contribute {
            key: key.into(),
            value: AnyValue::new(value),
        })
    }

    /// The attempt emits an event of this level with this message, as the step's reporter
    /// would.
    ///
    /// # Panics
    ///
    /// When the step declares no reporter, so the real step could not emit.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::event::{Event, Level};
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::journey::StatusKind;
    /// use itinera::report::RecordingReporter;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// # struct Charge<'a> {
    /// #     contributor: itinera::journey::Contributor<'a>,
    /// #     reporter: itinera::step::StepReporter<'a>,
    /// # }
    /// # impl itinera::step::Step for Charge<'_> {
    /// #     fn run(mut self) -> Result<Outcome, itinera::error::Error> {
    /// #         self.reporter.info("charging")?;
    /// #         self.contributor.contribute("charged", 40_i64);
    /// #         Ok(Outcome::success())
    /// #     }
    /// # }
    /// # struct ChargeFactory;
    /// # impl itinera::step::StepFactory for ChargeFactory {
    /// #     type Step<'a> = Charge<'a>;
    /// #     fn needs(&self) -> itinera::step::StepNeeds {
    /// #         itinera::step::StepNeeds::new().contributor().reporter()
    /// #     }
    /// #     fn build<'a>(
    /// #         &'a self,
    /// #         got: &mut itinera::step::Resolved<'a>,
    /// #     ) -> Result<Charge<'a>, itinera::error::Error> {
    /// #         let contributor = got.contributor()?;
    /// #         let reporter = got.reporter()?;
    /// #         Ok(Charge { contributor, reporter })
    /// #     }
    /// # }
    /// // ChargeFactory builds a charge step that declares a contributor and a reporter.
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), ChargeFactory))
    ///     .build()?;
    /// let recorder = RecordingReporter::default();
    /// let run = orders
    ///     .instance(Orders)
    ///     .reporter(recorder.clone())
    ///     .create()?
    ///     .scripted()
    ///     .step("charge")
    ///     .will_emit(Level::Warning, "slow gateway")
    ///     .will_succeed()
    ///     .run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
    /// let kinds: Vec<_> = recorder.events().iter().map(Event::kind).collect();
    /// assert!(kinds.contains(&"step_warning"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn will_emit(
        self,
        level: Level,
        message: impl Into<String>,
    ) -> AttemptScripting<I, M, Acting> {
        self.acting(Action::Emit {
            level,
            message: message.into(),
            data: None,
        })
    }

    /// The attempt emits an event of this level with this message and data, as the step's
    /// reporter would.
    ///
    /// # Panics
    ///
    /// When the step declares no reporter, so the real step could not emit.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::event::{Event, Level};
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::journey::StatusKind;
    /// use itinera::report::RecordingReporter;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// # struct Charge<'a> {
    /// #     contributor: itinera::journey::Contributor<'a>,
    /// #     reporter: itinera::step::StepReporter<'a>,
    /// # }
    /// # impl itinera::step::Step for Charge<'_> {
    /// #     fn run(mut self) -> Result<Outcome, itinera::error::Error> {
    /// #         self.reporter.info("charging")?;
    /// #         self.contributor.contribute("charged", 40_i64);
    /// #         Ok(Outcome::success())
    /// #     }
    /// # }
    /// # struct ChargeFactory;
    /// # impl itinera::step::StepFactory for ChargeFactory {
    /// #     type Step<'a> = Charge<'a>;
    /// #     fn needs(&self) -> itinera::step::StepNeeds {
    /// #         itinera::step::StepNeeds::new().contributor().reporter()
    /// #     }
    /// #     fn build<'a>(
    /// #         &'a self,
    /// #         got: &mut itinera::step::Resolved<'a>,
    /// #     ) -> Result<Charge<'a>, itinera::error::Error> {
    /// #         let contributor = got.contributor()?;
    /// #         let reporter = got.reporter()?;
    /// #         Ok(Charge { contributor, reporter })
    /// #     }
    /// # }
    /// // ChargeFactory builds a charge step that declares a contributor and a reporter.
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), ChargeFactory))
    ///     .build()?;
    /// let recorder = RecordingReporter::default();
    /// let run = orders
    ///     .instance(Orders)
    ///     .reporter(recorder.clone())
    ///     .create()?
    ///     .scripted()
    ///     .step("charge")
    ///     .will_emit_with(Level::Info, "charging", 40_i64)
    ///     .will_succeed()
    ///     .run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
    /// let kinds: Vec<_> = recorder.events().iter().map(Event::kind).collect();
    /// assert!(kinds.contains(&"step_info"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn will_emit_with(
        self,
        level: Level,
        message: impl Into<String>,
        data: impl Value,
    ) -> AttemptScripting<I, M, Acting> {
        self.acting(Action::Emit {
            level,
            message: message.into(),
            data: Some(AnyValue::new(data)),
        })
    }

    /// The attempt sets the entry `key` of the journey's global metadata to `value`, as the
    /// step's reporter would.
    ///
    /// # Panics
    ///
    /// When the step declares no reporter, so the real step could not set global metadata.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::journey::StatusKind;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// # struct Charge<'a> {
    /// #     contributor: itinera::journey::Contributor<'a>,
    /// #     reporter: itinera::step::StepReporter<'a>,
    /// # }
    /// # impl itinera::step::Step for Charge<'_> {
    /// #     fn run(mut self) -> Result<Outcome, itinera::error::Error> {
    /// #         self.reporter.info("charging")?;
    /// #         self.contributor.contribute("charged", 40_i64);
    /// #         Ok(Outcome::success())
    /// #     }
    /// # }
    /// # struct ChargeFactory;
    /// # impl itinera::step::StepFactory for ChargeFactory {
    /// #     type Step<'a> = Charge<'a>;
    /// #     fn needs(&self) -> itinera::step::StepNeeds {
    /// #         itinera::step::StepNeeds::new().contributor().reporter()
    /// #     }
    /// #     fn build<'a>(
    /// #         &'a self,
    /// #         got: &mut itinera::step::Resolved<'a>,
    /// #     ) -> Result<Charge<'a>, itinera::error::Error> {
    /// #         let contributor = got.contributor()?;
    /// #         let reporter = got.reporter()?;
    /// #         Ok(Charge { contributor, reporter })
    /// #     }
    /// # }
    /// // ChargeFactory builds a charge step that declares a contributor and a reporter.
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), ChargeFactory))
    ///     .build()?;
    /// let run = orders
    ///     .instance(Orders)
    ///     .create()?
    ///     .scripted()
    ///     .step("charge")
    ///     .will_set_global_metadata("tracking", "TRK-1".to_owned())
    ///     .will_succeed()
    ///     .run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn will_set_global_metadata(
        self,
        key: impl Into<String>,
        value: impl Value,
    ) -> AttemptScripting<I, M, Acting> {
        self.acting(Action::SetGlobalMetadata {
            key: key.into(),
            value: AnyValue::new(value),
        })
    }

    /// The attempt succeeds.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::journey::StatusKind;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(
    ///         StepDescriptor::new(step_name!("charge"), || {
    ///             Ok(Outcome::failure("declined".into()))
    ///         }),
    ///     )
    ///     .build()?;
    /// let run = orders.instance(Orders).create()?.scripted().step("charge").will_succeed().run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn will_succeed(self) -> StepScripting<I, M> {
        self.ending(Ending::Success)
    }

    /// The attempt fails for this reason, and suggests no retry. A text is the reason's code.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::journey::StatusKind;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    ///     .build()?;
    /// let run = orders
    ///     .instance(Orders)
    ///     .create()?
    ///     .scripted()
    ///     .step("charge")
    ///     .will_fail("declined")
    ///     .run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Failed);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn will_fail(self, reason: impl Into<Reason>) -> StepScripting<I, M> {
        self.ending(Ending::Failure(reason.into()))
    }

    /// The attempt fails for this reason, and suggests retrying the step. A text is the reason's
    /// code.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::journey::StatusKind;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(
    ///         StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success()))
    ///             .retry_budget(1),
    ///     )
    ///     .build()?;
    /// let run = orders
    ///     .instance(Orders)
    ///     .create()?
    ///     .scripted()
    ///     .step("charge")
    ///     .on_every_attempt()
    ///     .will_fail_retriably("gateway-unavailable")
    ///     .run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Failed);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn will_fail_retriably(self, reason: impl Into<Reason>) -> StepScripting<I, M> {
        self.ending(Ending::RetriableFailure(reason.into()))
    }

    /// The attempt has nothing to do, and the step is skipped.
    ///
    /// # Panics
    ///
    /// When the step is a rescue step, which cannot skip.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::journey::StatusKind;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    ///     .build()?;
    /// let run = orders.instance(Orders).create()?.scripted().step("charge").will_skip().run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn will_skip(self) -> StepScripting<I, M> {
        self.skipping(None)
    }

    /// The attempt has nothing to do, for this reason, and the step is skipped. A text is the
    /// reason's code.
    ///
    /// # Panics
    ///
    /// When the step is a rescue step, which cannot skip.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::journey::StatusKind;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    ///     .build()?;
    /// let run = orders
    ///     .instance(Orders)
    ///     .create()?
    ///     .scripted()
    ///     .step("charge")
    ///     .will_skip_because("already-paid")
    ///     .run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn will_skip_because(self, reason: impl Into<Reason>) -> StepScripting<I, M> {
        self.skipping(Some(reason.into()))
    }

    /// The attempt terminates abnormally, with an error carrying this message, as a step whose
    /// `run` returns an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::journey::StatusKind;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    ///     .build()?;
    /// let run = orders
    ///     .instance(Orders)
    ///     .create()?
    ///     .scripted()
    ///     .step("charge")
    ///     .will_terminate_abnormally("the ledger is closed")
    ///     .run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Failed);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn will_terminate_abnormally(self, message: impl Into<String>) -> StepScripting<I, M> {
        self.ending(Ending::AbnormalTermination(message.into()))
    }

    fn acting(mut self, action: Action) -> AttemptScripting<I, M, Acting> {
        self.step.target().check(&action);
        self.actions.push(action);
        AttemptScripting {
            step: self.step,
            slot: self.slot,
            actions: self.actions,
            state: PhantomData,
        }
    }

    #[expect(
        clippy::panic,
        reason = "a rescue step that skips is a mistake in a test's script"
    )]
    fn skipping(self, reason: Option<Reason>) -> StepScripting<I, M> {
        if self.step.target().is_rescue() {
            panic!(
                "{} is a rescue step, which cannot skip",
                self.step.target().name
            );
        }
        self.ending(Ending::Skipped(reason))
    }

    fn ending(self, ending: Ending) -> StepScripting<I, M> {
        let played = Played {
            actions: self.actions,
            ending,
        };
        self.step.scripted(self.slot, AttemptScript::Played(played))
    }

    fn real_run(self) -> StepScripting<I, M> {
        self.step.scripted(self.slot, AttemptScript::RealRun)
    }
}

impl<I: WorkflowInstance, M> AttemptScripting<I, M, Named> {
    /// Says what the attempt with this number does instead: attempt 1 when the step was just
    /// named.
    ///
    /// # Panics
    ///
    /// When `attempt` is not 1, since a step's attempts are scripted from 1 without gaps.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::journey::StatusKind;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    ///     .build()?;
    /// let run = orders
    ///     .instance(Orders)
    ///     .create()?
    ///     .scripted()
    ///     .step("charge")
    ///     .on_attempt(1)
    ///     .will_succeed()
    ///     .run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn on_attempt(self, attempt: u32) -> AttemptScripting<I, M, Opened> {
        self.step.on_attempt(attempt)
    }

    /// Says what every attempt of the step does.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::journey::StatusKind;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(
    ///         StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success()))
    ///             .retry_budget(2),
    ///     )
    ///     .build()?;
    /// let run = orders
    ///     .instance(Orders)
    ///     .create()?
    ///     .scripted()
    ///     .step("charge")
    ///     .on_every_attempt()
    ///     .will_succeed()
    ///     .run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn on_every_attempt(self) -> AttemptScripting<I, M, Opened> {
        self.step.on_every_attempt()
    }

    /// Attempt 1 runs the real step, as if the script did not name it.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::journey::StatusKind;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    ///     .build()?;
    /// let run = orders
    ///     .instance(Orders)
    ///     .create()?
    ///     .scripted()
    ///     .ensure_all_scripted()
    ///     .step("charge")
    ///     .will_run_for_real()
    ///     .run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn will_run_for_real(self) -> StepScripting<I, M> {
        self.real_run()
    }
}

impl<I: WorkflowInstance, M> AttemptScripting<I, M, Opened> {
    /// The attempts run the real step, as if the script did not name them.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::instance::WorkflowInstance;
    /// use itinera::journey::StatusKind;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(
    ///         StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success()))
    ///             .retry_budget(1),
    ///     )
    ///     .build()?;
    /// let run = orders
    ///     .instance(Orders)
    ///     .create()?
    ///     .scripted()
    ///     .step("charge")
    ///     .will_fail_retriably("declined")
    ///     .on_attempt(2)
    ///     .will_run_for_real()
    ///     .run()?;
    /// assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn will_run_for_real(self) -> StepScripting<I, M> {
        self.real_run()
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::rescued;

    #[test]
    #[should_panic(expected = "charge_rescue is a rescue step, which cannot skip")]
    fn a_rescue_step_cannot_be_scripted_to_skip() {
        rescued().step("charge_rescue").will_skip();
    }
}
