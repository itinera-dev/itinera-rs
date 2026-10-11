//! What a hook receives when it is called.

use std::any;
use std::marker::PhantomData;
use std::num::NonZeroU32;

use super::Answers;
use crate::error::Error;
use crate::journey::{Contributor, DataBagAccess, JourneyId};
use crate::mode::Synchronous;
use crate::policy::{
    ErrorHookKind, FailureHookKind, HookKind, HookReporter, InputAdapter, PolicyHookKind, Provides,
    RetryCause, StepFailure, StepFailureCause, StepHookKind, StepRetry,
};
use crate::step::{Input, OptionalInput, Reason, Reporting, StepName};
use crate::value::{AnyValue, Value};

/// What a hook receives when it is called: each thing it declared in its [`HookNeeds`], already
/// resolved, and what the executor tells every hook of its kind `H`, such as the step's name for
/// a step hook. A hook takes each declared value once.
///
/// Everything taken from it is the hook's own copy: changing it changes neither the data bag
/// nor what any other step or hook receives.
///
/// [`HookNeeds`]: super::HookNeeds
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::policy::{HookNeeds, OnStepSuccess, OnSuccess, Requested, StepSuccess};
/// use itinera::step::Input;
///
/// const RECEIPT: Input<String> = Input::new("receipt");
///
/// struct Audit;
///
/// impl<W: Send + Sync + 'static> OnStepSuccess<W> for Audit {
///     fn needs() -> HookNeeds<StepSuccess> {
///         HookNeeds::new().from_step(&RECEIPT).reporter()
///     }
///
///     fn on_step_success(
///         &self,
///         mut got: Requested<'_, W, StepSuccess>,
///     ) -> Result<Option<OnSuccess>, Error> {
///         let receipt = got.from_step(&RECEIPT)?;
///         got.reporter()?.info(format!("{} issued {receipt}", got.step_name()))?;
///         Ok(None)
///     }
/// }
/// ```
#[derive(derive_more::Debug)]
pub struct Requested<'a, W, H: HookKind, M = Synchronous> {
    #[debug(skip)]
    workflow: &'a W,
    journey_id: &'a JourneyId,
    context: H::Context,
    answers: Answers<'a>,
    #[debug(skip)]
    mode: PhantomData<fn() -> M>,
}

impl<'a, W, H: HookKind, M> Requested<'a, W, H, M> {
    pub(crate) fn new(
        workflow: &'a W,
        journey_id: &'a JourneyId,
        context: H::Context,
        answers: Answers<'a>,
    ) -> Self {
        Self {
            workflow,
            journey_id,
            context,
            answers,
            mode: PhantomData,
        }
    }

    pub(crate) fn take_reporting(&mut self) -> Result<Reporting<'a>, Error> {
        self.answers
            .reporting
            .take()
            .ok_or_else(|| undeclared_handle("a reporter"))
    }

    /// Takes the data from the workflow the hook declared under the input's key.
    ///
    /// # Errors
    ///
    /// When the hook did not declare it as required with this type, or took it already.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{HookNeeds, OnWorkflowSuccess, Requested, WorkflowSuccess};
    /// use itinera::step::Input;
    ///
    /// const AMOUNT: Input<i64> = Input::new("amount");
    ///
    /// struct Totals;
    ///
    /// impl<W: Send + Sync + 'static> OnWorkflowSuccess<W> for Totals {
    ///     fn needs() -> HookNeeds<WorkflowSuccess> {
    ///         HookNeeds::new().from_workflow(&AMOUNT)
    ///     }
    ///
    ///     fn on_workflow_success(
    ///         &self,
    ///         mut got: Requested<'_, W, WorkflowSuccess>,
    ///     ) -> Result<(), Error> {
    ///         let amount: i64 = got.from_workflow(&AMOUNT)?;
    ///         println!("sold for {amount}");
    ///         Ok(())
    ///     }
    /// }
    /// ```
    pub fn from_workflow<T: Value>(&mut self, input: &Input<T>) -> Result<T, Error> {
        self.answers
            .from_workflow
            .take(input.key())
            .and_then(AnyValue::downcast)
            .ok_or_else(|| undeclared::<T>("data from the workflow", input.key()))
    }

    /// Takes the optional data from the workflow the hook declared under the input's key: `None`
    /// when the data bag holds nothing there.
    ///
    /// # Errors
    ///
    /// When the hook did not declare it as optional with this type, or took it already.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{HookNeeds, OnWorkflowFailure, Requested, WorkflowFailure};
    /// use itinera::step::OptionalInput;
    ///
    /// const CUSTOMER: OptionalInput<String> = OptionalInput::new("customer");
    ///
    /// struct Apology;
    ///
    /// impl<W: Send + Sync + 'static> OnWorkflowFailure<W> for Apology {
    ///     fn needs() -> HookNeeds<WorkflowFailure> {
    ///         HookNeeds::new().optional_from_workflow(&CUSTOMER)
    ///     }
    ///
    ///     fn on_workflow_failure(
    ///         &self,
    ///         mut got: Requested<'_, W, WorkflowFailure>,
    ///     ) -> Result<(), Error> {
    ///         if let Some(customer) = got.optional_from_workflow(&CUSTOMER)? {
    ///             println!("sorry, {customer}");
    ///         }
    ///         Ok(())
    ///     }
    /// }
    /// ```
    pub fn optional_from_workflow<T: Value>(
        &mut self,
        input: &OptionalInput<T>,
    ) -> Result<Option<T>, Error> {
        optional(
            self.answers.from_workflow.take_optional(input.key()),
            "data from the workflow",
            input.key(),
        )
    }

    /// The ID of the journey the hook is called in.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{OnWorkflowSuccess, Requested, WorkflowSuccess};
    ///
    /// struct Receipt;
    ///
    /// impl<W: Send + Sync + 'static> OnWorkflowSuccess<W> for Receipt {
    ///     fn on_workflow_success(
    ///         &self,
    ///         got: Requested<'_, W, WorkflowSuccess>,
    ///     ) -> Result<(), Error> {
    ///         println!("journey {} succeeded", got.journey_id());
    ///         Ok(())
    ///     }
    /// }
    /// ```
    pub fn journey_id(&self) -> &'a JourneyId {
        self.journey_id
    }
}

impl<W, H: StepHookKind, M> Requested<'_, W, H, M> {
    /// Takes the data from the step the hook declared under the input's key: what the attempt
    /// contributed there, committed or not.
    ///
    /// # Errors
    ///
    /// When the hook did not declare it as required with this type, or took it already.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{HookNeeds, OnStepFailure, FailWorkflow, Requested, StepFailure};
    /// use itinera::step::Input;
    ///
    /// const DECLINE: Input<String> = Input::new("decline");
    ///
    /// struct Bank;
    ///
    /// impl<W: Send + Sync + 'static> OnStepFailure<W> for Bank {
    ///     fn needs() -> HookNeeds<StepFailure> {
    ///         HookNeeds::new().from_step(&DECLINE)
    ///     }
    ///
    ///     fn on_step_failure(
    ///         &self,
    ///         mut got: Requested<'_, W, StepFailure>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         println!("the bank declined: {}", got.from_step(&DECLINE)?);
    ///         Ok(None)
    ///     }
    /// }
    /// ```
    pub fn from_step<T: Value>(&mut self, input: &Input<T>) -> Result<T, Error> {
        self.answers
            .from_step
            .take(input.key())
            .and_then(AnyValue::downcast)
            .ok_or_else(|| undeclared::<T>("data from the step", input.key()))
    }

    /// Takes the optional data from the step the hook declared under the input's key: `None`
    /// when the attempt contributed nothing there.
    ///
    /// # Errors
    ///
    /// When the hook did not declare it as optional with this type, or took it already.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{FailWorkflow, HookNeeds, OnStepRetry, Requested, StepRetry};
    /// use itinera::step::OptionalInput;
    ///
    /// const DRAFT: OptionalInput<String> = OptionalInput::new("draft");
    ///
    /// struct Drafts;
    ///
    /// impl<W: Send + Sync + 'static> OnStepRetry<W> for Drafts {
    ///     fn needs() -> HookNeeds<StepRetry> {
    ///         HookNeeds::new().optional_from_step(&DRAFT)
    ///     }
    ///
    ///     fn on_step_retry(
    ///         &self,
    ///         mut got: Requested<'_, W, StepRetry>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         if let Some(draft) = got.optional_from_step(&DRAFT)? {
    ///             println!("dropping the draft {draft}");
    ///         }
    ///         Ok(None)
    ///     }
    /// }
    /// ```
    pub fn optional_from_step<T: Value>(
        &mut self,
        input: &OptionalInput<T>,
    ) -> Result<Option<T>, Error> {
        optional(
            self.answers.from_step.take_optional(input.key()),
            "data from the step",
            input.key(),
        )
    }

    /// The name of the step the hook acts on.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{OnStepSuccess, OnSuccess, Requested, StepSuccess};
    ///
    /// struct Trace;
    ///
    /// impl<W: Send + Sync + 'static> OnStepSuccess<W> for Trace {
    ///     fn on_step_success(
    ///         &self,
    ///         got: Requested<'_, W, StepSuccess>,
    ///     ) -> Result<Option<OnSuccess>, Error> {
    ///         println!("{} succeeded", got.step_name());
    ///         Ok(None)
    ///     }
    /// }
    /// ```
    pub fn step_name(&self) -> StepName {
        H::attempt(&self.context).step
    }

    /// The number of the attempt the hook acts on, from 1.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{OnStepSuccess, OnSuccess, Requested, StepSuccess};
    ///
    /// struct Trace;
    ///
    /// impl<W: Send + Sync + 'static> OnStepSuccess<W> for Trace {
    ///     fn on_step_success(
    ///         &self,
    ///         got: Requested<'_, W, StepSuccess>,
    ///     ) -> Result<Option<OnSuccess>, Error> {
    ///         println!("succeeded at attempt {}", got.attempt());
    ///         Ok(None)
    ///     }
    /// }
    /// ```
    pub fn attempt(&self) -> NonZeroU32 {
        H::attempt(&self.context).attempt
    }
}

impl<W, H: FailureHookKind, M> Requested<'_, W, H, M> {
    /// Takes the reason of the attempt's failure, which the hook declared as required.
    ///
    /// # Errors
    ///
    /// When the hook did not declare the reason as required, or took it already.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{FailWorkflow, HookNeeds, OnStepFailure, Requested, StepFailure};
    ///
    /// struct Alarm;
    ///
    /// impl<W: Send + Sync + 'static> OnStepFailure<W> for Alarm {
    ///     fn needs() -> HookNeeds<StepFailure> {
    ///         HookNeeds::new().reason()
    ///     }
    ///
    ///     fn on_step_failure(
    ///         &self,
    ///         mut got: Requested<'_, W, StepFailure>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         println!("failed with {}", got.reason()?.code());
    ///         Ok(None)
    ///     }
    /// }
    /// ```
    pub fn reason(&mut self) -> Result<Reason, Error> {
        self.answers
            .reason
            .take()
            .flatten()
            .ok_or_else(|| undeclared_handle("the failure reason as required"))
    }

    /// Takes the reason of the attempt's failure, which the hook declared as optional: `None`
    /// when the attempt reported no failure.
    ///
    /// # Errors
    ///
    /// When the hook did not declare the reason, or took it already.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{FailWorkflow, HookNeeds, OnStepRetry, Requested, StepRetry};
    ///
    /// struct Watch;
    ///
    /// impl<W: Send + Sync + 'static> OnStepRetry<W> for Watch {
    ///     fn needs() -> HookNeeds<StepRetry> {
    ///         HookNeeds::new().optional_reason()
    ///     }
    ///
    ///     fn on_step_retry(
    ///         &self,
    ///         mut got: Requested<'_, W, StepRetry>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         if got.optional_reason()?.is_none() {
    ///             println!("retrying after an abnormal termination");
    ///         }
    ///         Ok(None)
    ///     }
    /// }
    /// ```
    pub fn optional_reason(&mut self) -> Result<Option<Reason>, Error> {
        self.answers
            .reason
            .take()
            .ok_or_else(|| undeclared_handle("the failure reason"))
    }
}

impl<'a, W, H: ErrorHookKind, M> Requested<'a, W, H, M> {
    /// Takes the error that ended the attempt abnormally, which the hook declared as required.
    ///
    /// # Errors
    ///
    /// When the hook did not declare the error as required, or took it already.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{
    ///     FailWorkflow, HookNeeds, OnStepAbnormalTermination, Requested, StepAbnormalTermination,
    /// };
    ///
    /// struct Crashes;
    ///
    /// impl<W: Send + Sync + 'static> OnStepAbnormalTermination<W> for Crashes {
    ///     fn needs() -> HookNeeds<StepAbnormalTermination> {
    ///         HookNeeds::new().error()
    ///     }
    ///
    ///     fn on_step_abnormal_termination(
    ///         &self,
    ///         mut got: Requested<'_, W, StepAbnormalTermination>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         println!("crashed: {}", got.error()?);
    ///         Ok(None)
    ///     }
    /// }
    /// ```
    pub fn error(&mut self) -> Result<&'a Error, Error> {
        self.answers
            .error
            .take()
            .flatten()
            .ok_or_else(|| undeclared_handle("the error as required"))
    }

    /// Takes the error that ended the attempt abnormally, which the hook declared as optional:
    /// `None` when the attempt did not end in an abnormal termination.
    ///
    /// # Errors
    ///
    /// When the hook did not declare the error, or took it already.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{FailWorkflow, HookNeeds, OnStepFailure, Requested, StepFailure};
    ///
    /// struct Alarm;
    ///
    /// impl<W: Send + Sync + 'static> OnStepFailure<W> for Alarm {
    ///     fn needs() -> HookNeeds<StepFailure> {
    ///         HookNeeds::new().optional_error()
    ///     }
    ///
    ///     fn on_step_failure(
    ///         &self,
    ///         mut got: Requested<'_, W, StepFailure>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         if let Some(error) = got.optional_error()? {
    ///             println!("crashed: {error}");
    ///         }
    ///         Ok(None)
    ///     }
    /// }
    /// ```
    pub fn optional_error(&mut self) -> Result<Option<&'a Error>, Error> {
        self.answers
            .error
            .take()
            .ok_or_else(|| undeclared_handle("the error"))
    }
}

impl<W, M> Requested<'_, W, StepFailure, M> {
    /// Why the step was given up.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{
    ///     FailWorkflow, OnStepFailure, Requested, StepFailure, StepFailureCause,
    /// };
    ///
    /// struct Patience;
    ///
    /// impl<W: Send + Sync + 'static> OnStepFailure<W> for Patience {
    ///     fn on_step_failure(
    ///         &self,
    ///         got: Requested<'_, W, StepFailure>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         if got.cause() == StepFailureCause::RetriesExhausted {
    ///             println!("gave up after {} attempts", got.attempt());
    ///         }
    ///         Ok(None)
    ///     }
    /// }
    /// ```
    pub fn cause(&self) -> StepFailureCause {
        self.context.1
    }
}

impl<W, M> Requested<'_, W, StepRetry, M> {
    /// Why the step is attempted again.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::RetryCause;
    /// use itinera::policy::{FailWorkflow, OnStepRetry, Requested, StepRetry};
    ///
    /// struct Crashes;
    ///
    /// impl<W: Send + Sync + 'static> OnStepRetry<W> for Crashes {
    ///     fn on_step_retry(
    ///         &self,
    ///         got: Requested<'_, W, StepRetry>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         if got.cause() == RetryCause::AbnormalTermination {
    ///             println!("retrying after a crash");
    ///         }
    ///         Ok(None)
    ///     }
    /// }
    /// ```
    pub fn cause(&self) -> RetryCause {
        self.context.1
    }
}

impl<'a, W, H: PolicyHookKind, M> Requested<'a, W, H, M> {
    /// Takes the contributor the hook declared. What it contributes is committed to the data bag
    /// once the hook returns without failing.
    ///
    /// # Errors
    ///
    /// When the hook did not declare a contributor, or took it already.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{HookNeeds, OnWorkflowSuccess, Requested, WorkflowSuccess};
    ///
    /// struct Stamp;
    ///
    /// impl<W: Send + Sync + 'static> OnWorkflowSuccess<W> for Stamp {
    ///     fn needs() -> HookNeeds<WorkflowSuccess> {
    ///         HookNeeds::new().contributor()
    ///     }
    ///
    ///     fn on_workflow_success(
    ///         &self,
    ///         mut got: Requested<'_, W, WorkflowSuccess>,
    ///     ) -> Result<(), Error> {
    ///         got.contributor()?.contribute("closed", true);
    ///         Ok(())
    ///     }
    /// }
    /// ```
    pub fn contributor(&mut self) -> Result<Contributor<'a>, Error> {
        self.answers
            .contributor
            .take()
            .ok_or_else(|| undeclared_handle("a contributor"))
    }

    /// The workflow, as the role `R` it provides.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{OnWorkflowFailure, Provides, Requested, WorkflowFailure};
    ///
    /// trait Notifier {
    ///     fn notify(&self, message: &str) -> Result<(), Error>;
    /// }
    ///
    /// struct Notify;
    ///
    /// impl<W> OnWorkflowFailure<W> for Notify
    /// where
    ///     W: Provides<dyn Notifier> + Send + Sync + 'static,
    /// {
    ///     fn on_workflow_failure(
    ///         &self,
    ///         got: Requested<'_, W, WorkflowFailure>,
    ///     ) -> Result<(), Error> {
    ///         got.role::<dyn Notifier>().notify("the order failed")
    ///     }
    /// }
    /// ```
    pub fn role<R: ?Sized>(&self) -> &'a R
    where
        W: Provides<R>,
    {
        self.workflow.role()
    }
}

impl<'a, W, H: PolicyHookKind> Requested<'a, W, H, Synchronous> {
    /// Takes the reporter the hook declared.
    ///
    /// # Errors
    ///
    /// When the hook did not declare a reporter, or took it already.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{HookNeeds, OnWorkflowFailure, Requested, WorkflowFailure};
    ///
    /// struct Alert;
    ///
    /// impl<W: Send + Sync + 'static> OnWorkflowFailure<W> for Alert {
    ///     fn needs() -> HookNeeds<WorkflowFailure> {
    ///         HookNeeds::new().reporter()
    ///     }
    ///
    ///     fn on_workflow_failure(
    ///         &self,
    ///         mut got: Requested<'_, W, WorkflowFailure>,
    ///     ) -> Result<(), Error> {
    ///         got.reporter()?.error("the order failed")?;
    ///         Ok(())
    ///     }
    /// }
    /// ```
    pub fn reporter(&mut self) -> Result<HookReporter<'a>, Error> {
        self.take_reporting().map(HookReporter::from)
    }
}

impl<'a, W> Requested<'a, W, InputAdapter> {
    /// The name of the step being built.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{InputAdapter, Requested};
    /// use itinera::value::AnyValue;
    ///
    /// struct Orders;
    ///
    /// impl Orders {
    ///     fn pricing(&self, got: Requested<'_, Self, InputAdapter>) -> Result<Option<AnyValue>, Error> {
    ///         println!("building {}", got.step_name());
    ///         Ok(None)
    ///     }
    /// }
    /// ```
    pub fn step_name(&self) -> StepName {
        let (step, _) = self.context;
        step
    }

    /// The key of the input being resolved.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{InputAdapter, Requested};
    /// use itinera::value::AnyValue;
    ///
    /// struct Orders {
    ///     price: i64,
    /// }
    ///
    /// impl Orders {
    ///     fn pricing(&self, got: Requested<'_, Self, InputAdapter>) -> Result<Option<AnyValue>, Error> {
    ///         Ok((got.key() == "amount").then(|| AnyValue::new(self.price)))
    ///     }
    /// }
    /// ```
    pub fn key(&self) -> &'static str {
        let (_, key) = self.context;
        key
    }

    /// Takes the read access to the data bag the adapter declared, valid for this call only.
    ///
    /// # Errors
    ///
    /// When the adapter did not declare it, or took it already.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::journey::Read;
    /// use itinera::policy::{InputAdapter, Requested};
    /// use itinera::value::AnyValue;
    ///
    /// struct Orders {
    ///     price: i64,
    /// }
    ///
    /// impl Orders {
    ///     fn pricing(&self, mut got: Requested<'_, Self, InputAdapter>) -> Result<Option<AnyValue>, Error> {
    ///         match got.data_bag()?.read::<i64>("quantity") {
    ///             Read::Present(quantity) => Ok(Some(AnyValue::new(self.price * quantity))),
    ///             Read::Absent | Read::OtherType => Ok(None),
    ///         }
    ///     }
    /// }
    /// ```
    pub fn data_bag(&mut self) -> Result<DataBagAccess<'a>, Error> {
        self.answers.data_bag.take().ok_or_else(undeclared_data_bag)
    }
}

/// Reads an optional value a hook declared, which the executor resolved as present or absent.
fn optional<T: Value>(
    slot: Option<Option<AnyValue>>,
    what: &str,
    key: &str,
) -> Result<Option<T>, Error> {
    match slot {
        Some(None) => Ok(None),
        Some(Some(value)) => value
            .downcast()
            .map(Some)
            .ok_or_else(|| undeclared::<T>(what, key)),
        None => Err(undeclared::<T>(what, key)),
    }
}

fn undeclared<T>(what: &str, key: &str) -> Error {
    Error::msg(format!(
        "the hook does not declare {what} \"{key}\" of type {}, or took it already",
        any::type_name::<T>()
    ))
}

fn undeclared_data_bag() -> Error {
    Error::msg("the input adapter does not declare read access to the data bag, or took it already")
}

fn undeclared_handle(what: &str) -> Error {
    Error::msg(format!(
        "the hook does not declare {what}, or took it already"
    ))
}
