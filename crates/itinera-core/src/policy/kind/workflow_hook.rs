//! The kinds of the two workflow hooks.

use super::{HookKind, PolicyHookKind, sealed};

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

impl sealed::Sealed for WorkflowSuccess {}
impl sealed::Sealed for WorkflowFailure {}

impl HookKind for WorkflowSuccess {
    type Context = ();
}

impl HookKind for WorkflowFailure {
    type Context = ();
}

impl PolicyHookKind for WorkflowSuccess {}
impl PolicyHookKind for WorkflowFailure {}
