//! The kinds of hook, which decide what a hook may request.

use std::fmt;

use super::{RetryCause, StepFailureCause};
use crate::step::{StepAttempt, StepName};

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// A kind of hook. [`HookNeeds`] and [`Requested`] offer a hook only the requests its kind may
/// make, so a request it may not make does not compile.
///
/// Only itinera's own kinds implement it: one for each of the six hooks of policies, and one for
/// input adapters.
///
/// [`HookNeeds`]: super::HookNeeds
/// [`Requested`]: super::Requested
///
/// # Examples
///
/// ```
/// use itinera::policy::{HookKind, HookNeeds, WorkflowSuccess};
///
/// fn nothing<H: HookKind>() -> HookNeeds<H> {
///     HookNeeds::new()
/// }
///
/// let needs: HookNeeds<WorkflowSuccess> = nothing();
/// ```
pub trait HookKind: sealed::Sealed + Copy + Send + Sync + 'static {
    /// What the executor tells a hook of this kind about its call.
    #[doc(hidden)]
    type Context: Clone + fmt::Debug + Send + Sync;
}

/// A step hook: it acts on one attempt of a step, so it may request data from the step, the
/// step's name and the attempt number.
///
/// # Examples
///
/// ```
/// use itinera::policy::{HookNeeds, StepHookKind, StepSuccess};
/// use itinera::step::Input;
///
/// const RECEIPT: Input<String> = Input::new("receipt");
///
/// fn the_receipt<H: StepHookKind>() -> HookNeeds<H> {
///     HookNeeds::new().from_step(&RECEIPT)
/// }
///
/// let needs: HookNeeds<StepSuccess> = the_receipt();
/// ```
pub trait StepHookKind: HookKind {
    /// The attempt the hook acts on.
    #[doc(hidden)]
    fn attempt(context: &Self::Context) -> &StepAttempt;
}

/// A step hook called because an attempt did not succeed, `on step failure` or `on step retry`:
/// it may request the failure's reason.
///
/// # Examples
///
/// ```
/// use itinera::policy::{FailureHookKind, HookNeeds, StepRetry};
///
/// fn the_reason<H: FailureHookKind>() -> HookNeeds<H> {
///     HookNeeds::new().optional_reason()
/// }
///
/// let needs: HookNeeds<StepRetry> = the_reason();
/// ```
pub trait FailureHookKind: StepHookKind {}

/// A step hook that may request the error of an abnormal termination: `on step failure`,
/// `on step retry` and `on step abnormal termination`.
///
/// # Examples
///
/// ```
/// use itinera::policy::{ErrorHookKind, HookNeeds, StepAbnormalTermination};
///
/// fn the_error<H: ErrorHookKind>() -> HookNeeds<H> {
///     HookNeeds::new().error()
/// }
///
/// let needs: HookNeeds<StepAbnormalTermination> = the_error();
/// ```
pub trait ErrorHookKind: StepHookKind {}

/// A hook of a policy, step hook or workflow hook: it may request roles, a contributor and a
/// reporter, which input adapters may not.
///
/// # Examples
///
/// ```
/// use itinera::policy::{HookNeeds, PolicyHookKind, WorkflowFailure};
///
/// fn writing<H: PolicyHookKind>() -> HookNeeds<H> {
///     HookNeeds::new().contributor().reporter()
/// }
///
/// let needs: HookNeeds<WorkflowFailure> = writing();
/// ```
pub trait PolicyHookKind: HookKind {}

/// `on step success`, called after an attempt that succeeded.
///
/// # Examples
///
/// ```
/// use itinera::policy::{HookNeeds, StepSuccess};
///
/// let needs = HookNeeds::<StepSuccess>::new().reporter();
/// ```
#[derive(Clone, Copy, Debug)]
pub enum StepSuccess {}

/// `on step failure`, called after a step was given up.
///
/// # Examples
///
/// ```
/// use itinera::policy::{HookNeeds, StepFailure};
///
/// let needs = HookNeeds::<StepFailure>::new().reason();
/// ```
#[derive(Clone, Copy, Debug)]
pub enum StepFailure {}

/// `on step retry`, called before a step is attempted again.
///
/// # Examples
///
/// ```
/// use itinera::policy::{HookNeeds, StepRetry};
///
/// let needs = HookNeeds::<StepRetry>::new().optional_error();
/// ```
#[derive(Clone, Copy, Debug)]
pub enum StepRetry {}

/// `on step abnormal termination`, called after an attempt ended in an abnormal termination.
///
/// # Examples
///
/// ```
/// use itinera::policy::{HookNeeds, StepAbnormalTermination};
///
/// let needs = HookNeeds::<StepAbnormalTermination>::new().error();
/// ```
#[derive(Clone, Copy, Debug)]
pub enum StepAbnormalTermination {}

/// `on workflow success`, called once the journey has succeeded.
///
/// # Examples
///
/// ```
/// use itinera::policy::{HookNeeds, WorkflowSuccess};
///
/// let needs = HookNeeds::<WorkflowSuccess>::new().contributor();
/// ```
#[derive(Clone, Copy, Debug)]
pub enum WorkflowSuccess {}

/// `on workflow failure`, called once the journey has failed.
///
/// # Examples
///
/// ```
/// use itinera::policy::{HookNeeds, WorkflowFailure};
///
/// let needs = HookNeeds::<WorkflowFailure>::new().reporter();
/// ```
#[derive(Clone, Copy, Debug)]
pub enum WorkflowFailure {}

/// `input adapter`, a hook of the workflow itself, called for each input of a step it is
/// attached to while the step is built. It may request data from the workflow, and nothing a
/// policy's hook may request beyond that.
///
/// # Examples
///
/// ```
/// use itinera::policy::{HookNeeds, InputAdapter};
/// use itinera::step::Input;
///
/// const PRICE: Input<i64> = Input::new("price");
///
/// let needs = HookNeeds::<InputAdapter>::new().from_workflow(&PRICE);
/// ```
#[derive(Clone, Copy, Debug)]
pub enum InputAdapter {}

impl sealed::Sealed for StepSuccess {}
impl sealed::Sealed for StepFailure {}
impl sealed::Sealed for StepRetry {}
impl sealed::Sealed for StepAbnormalTermination {}
impl sealed::Sealed for WorkflowSuccess {}
impl sealed::Sealed for WorkflowFailure {}
impl sealed::Sealed for InputAdapter {}

impl HookKind for StepSuccess {
    type Context = StepAttempt;
}

impl HookKind for StepFailure {
    type Context = (StepAttempt, StepFailureCause);
}

impl HookKind for StepRetry {
    type Context = (StepAttempt, RetryCause);
}

impl HookKind for StepAbnormalTermination {
    type Context = StepAttempt;
}

impl HookKind for WorkflowSuccess {
    type Context = ();
}

impl HookKind for WorkflowFailure {
    type Context = ();
}

impl HookKind for InputAdapter {
    /// The step being built, and the key of the input being resolved.
    type Context = (StepName, &'static str);
}

impl StepHookKind for StepSuccess {
    fn attempt(context: &StepAttempt) -> &StepAttempt {
        context
    }
}

impl StepHookKind for StepFailure {
    fn attempt((attempt, _): &(StepAttempt, StepFailureCause)) -> &StepAttempt {
        attempt
    }
}

impl StepHookKind for StepRetry {
    fn attempt((attempt, _): &(StepAttempt, RetryCause)) -> &StepAttempt {
        attempt
    }
}

impl StepHookKind for StepAbnormalTermination {
    fn attempt(context: &StepAttempt) -> &StepAttempt {
        context
    }
}

impl FailureHookKind for StepFailure {}
impl FailureHookKind for StepRetry {}

impl ErrorHookKind for StepFailure {}
impl ErrorHookKind for StepRetry {}
impl ErrorHookKind for StepAbnormalTermination {}

impl PolicyHookKind for StepSuccess {}
impl PolicyHookKind for StepFailure {}
impl PolicyHookKind for StepRetry {}
impl PolicyHookKind for StepAbnormalTermination {}
impl PolicyHookKind for WorkflowSuccess {}
impl PolicyHookKind for WorkflowFailure {}
