//! The kind of service providers.

use std::marker::PhantomData;

use super::{HookKind, ReportingHookKind, sealed};
use crate::service::ProviderScope;
use crate::step::StepAttempt;

/// `service provider`, a hook of the workflow itself, of scope `S`, called while a step it
/// serves is built. It may request data from the workflow and a reporter, and nothing else a
/// policy's hook may request; one of scope [`PerAttempt`] also knows the step and attempt it
/// serves.
///
/// [`PerAttempt`]: crate::service::PerAttempt
///
/// # Examples
///
/// ```
/// use itinera::policy::{HookNeeds, Provider};
/// use itinera::service::PerJourney;
/// use itinera::step::Input;
///
/// const GATEWAY: Input<String> = Input::new("gateway");
///
/// let needs = HookNeeds::<Provider<PerJourney>>::new()
///     .from_workflow(&GATEWAY)
///     .reporter();
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Provider<S> {
    scope: PhantomData<fn() -> S>,
}

impl<S: ProviderScope> sealed::Sealed for Provider<S> {}

impl<S: ProviderScope> HookKind for Provider<S> {
    /// The step being built, and its attempt.
    type Context = StepAttempt;
}

impl<S: ProviderScope> ReportingHookKind for Provider<S> {}
