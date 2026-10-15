//! What a hook declares it needs.

use std::marker::PhantomData;

use super::{Needs, Request};
use crate::policy::{
    ErrorHookKind, FailureHookKind, HookKind, PolicyHookKind, ReportingHookKind, StepHookKind,
};
use crate::step::{InputNeed, InputToken, Requirement};
use crate::workflow::InputAdapter;

/// Everything a hook declares it needs, which the executor resolves before the hook runs: data
/// from the step and from the workflow, the failure's reason, the error, and whether it needs a
/// contributor and a reporter.
///
/// Its kind `H` decides what it may declare: data from the step only for step hooks, the reason
/// only for `on step failure` and `on step retry`, the error only for those and
/// `on step abnormal termination`, a contributor only for hooks of policies, a reporter only for
/// those and service providers, and read access to the data bag only for input adapters.
///
/// The requests are resolved in the order they are declared, and the first required one without
/// a value aborts the journey before the hook runs. Declaring the reason or the error again
/// replaces the earlier declaration, and moves it to its new place in that order.
///
/// # Examples
///
/// ```
/// use itinera::policy::{HookNeeds, StepFailure};
/// use itinera::step::{Input, OptionalInput};
///
/// const DECLINE: Input<String> = Input::new("decline");
/// const CUSTOMER: OptionalInput<String> = OptionalInput::new("customer");
///
/// let needs = HookNeeds::<StepFailure>::new()
///     .from_step(&DECLINE)
///     .from_workflow(&CUSTOMER)
///     .reason()
///     .reporter();
/// ```
#[derive(Clone, derive_more::Debug, derive_more::Into)]
pub struct HookNeeds<H> {
    needs: Needs,
    #[debug(skip)]
    #[into(skip)]
    kind: PhantomData<fn() -> H>,
}

impl<H> Default for HookNeeds<H> {
    fn default() -> Self {
        Self {
            needs: Needs::default(),
            kind: PhantomData,
        }
    }
}

impl<H: HookKind> HookNeeds<H> {
    /// Needs nothing.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{HookNeeds, WorkflowSuccess};
    ///
    /// let needs = HookNeeds::<WorkflowSuccess>::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Needs data from the workflow: the value of the input's type under its key in the data bag.
    /// For an [`Input`], without it the journey is aborted with `required data missing` before the
    /// hook runs; for an [`OptionalInput`], the hook receives `None`, and `optional_input_absent`
    /// is emitted.
    ///
    /// [`Input`]: crate::step::Input
    /// [`OptionalInput`]: crate::step::OptionalInput
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{HookNeeds, WorkflowFailure};
    /// use itinera::step::{Input, OptionalInput};
    ///
    /// const AMOUNT: Input<i64> = Input::new("amount");
    /// const CUSTOMER: OptionalInput<String> = OptionalInput::new("customer");
    ///
    /// let needs = HookNeeds::<WorkflowFailure>::new()
    ///     .from_workflow(&AMOUNT)
    ///     .from_workflow(&CUSTOMER);
    /// ```
    pub fn from_workflow(mut self, input: &impl InputToken) -> Self {
        self.needs
            .request(Request::FromWorkflow(InputNeed::of(input)));
        self
    }
}

impl<H: StepHookKind> HookNeeds<H> {
    /// Needs data from the step: the value of the input's type the attempt contributed under the
    /// input's key, committed or not. For an [`Input`], without it the journey is aborted with
    /// `required data missing` before the hook runs; for an [`OptionalInput`], the hook receives
    /// `None`, and `optional_input_absent` is emitted.
    ///
    /// [`Input`]: crate::step::Input
    /// [`OptionalInput`]: crate::step::OptionalInput
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{HookNeeds, StepRetry};
    /// use itinera::step::{Input, OptionalInput};
    ///
    /// const RECEIPT: Input<String> = Input::new("receipt");
    /// const DRAFT: OptionalInput<String> = OptionalInput::new("draft");
    ///
    /// let needs = HookNeeds::<StepRetry>::new()
    ///     .from_step(&RECEIPT)
    ///     .from_step(&DRAFT);
    /// ```
    pub fn from_step(mut self, input: &impl InputToken) -> Self {
        self.needs.request(Request::FromStep(InputNeed::of(input)));
        self
    }
}

impl<H: FailureHookKind> HookNeeds<H> {
    /// Needs the reason of the attempt's failure. When the attempt reported no failure, because
    /// it ended in an abnormal termination, the journey is aborted with `required data missing`
    /// before the hook runs.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{HookNeeds, StepFailure};
    ///
    /// let needs = HookNeeds::<StepFailure>::new().reason();
    /// ```
    pub fn reason(mut self) -> Self {
        self.needs.request_reason(Requirement::Required);
        self
    }

    /// May use the reason of the attempt's failure, if it reported one.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{HookNeeds, StepRetry};
    ///
    /// let needs = HookNeeds::<StepRetry>::new().optional_reason();
    /// ```
    pub fn optional_reason(mut self) -> Self {
        self.needs.request_reason(Requirement::Optional);
        self
    }
}

impl<H: ErrorHookKind> HookNeeds<H> {
    /// Needs the error that ended the attempt abnormally. When the attempt did not end in an
    /// abnormal termination, the journey is aborted with `required data missing` before the hook
    /// runs.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{HookNeeds, StepAbnormalTermination};
    ///
    /// let needs = HookNeeds::<StepAbnormalTermination>::new().error();
    /// ```
    pub fn error(mut self) -> Self {
        self.needs.request_error(Requirement::Required);
        self
    }

    /// May use the error that ended the attempt abnormally, if it did.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{HookNeeds, StepFailure};
    ///
    /// let needs = HookNeeds::<StepFailure>::new().optional_error();
    /// ```
    pub fn optional_error(mut self) -> Self {
        self.needs.request_error(Requirement::Optional);
        self
    }
}

impl HookNeeds<InputAdapter> {
    /// Needs read access to the data bag, to read keys the adapter only knows when it is called.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::HookNeeds;
    /// use itinera::workflow::InputAdapter;
    ///
    /// let needs = HookNeeds::<InputAdapter>::new().data_bag();
    /// ```
    pub fn data_bag(mut self) -> Self {
        self.needs.data_bag = true;
        self
    }
}

impl<H: PolicyHookKind> HookNeeds<H> {
    /// Needs a contributor, to add data to the data bag. What the hook contributes is committed
    /// once it returns without failing.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{HookNeeds, StepSuccess};
    ///
    /// let needs = HookNeeds::<StepSuccess>::new().contributor();
    /// ```
    pub fn contributor(mut self) -> Self {
        self.needs.contributor = true;
        self
    }
}

impl<H: ReportingHookKind> HookNeeds<H> {
    /// Needs a reporter, to emit `journey_info`, `journey_warning` and `journey_error`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{HookNeeds, WorkflowFailure};
    ///
    /// let needs = HookNeeds::<WorkflowFailure>::new().reporter();
    /// ```
    pub fn reporter(mut self) -> Self {
        self.needs.reporter = true;
        self
    }
}
