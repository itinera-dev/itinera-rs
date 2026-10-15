//! The kind of input adapters.

use crate::policy::{HookKind, SealedHookKind};
use crate::step::StepName;

/// `input adapter`, a hook of the workflow itself, called for each input of a step it is
/// attached to while the step is built. It may request data from the workflow, and nothing a
/// policy's hook may request beyond that.
///
/// # Examples
///
/// ```
/// use itinera::policy::HookNeeds;
/// use itinera::step::Input;
/// use itinera::workflow::InputAdapter;
///
/// const PRICE: Input<i64> = Input::new("price");
///
/// let needs = HookNeeds::<InputAdapter>::new().from_workflow(&PRICE);
/// ```
#[derive(Clone, Copy, Debug)]
pub enum InputAdapter {}

impl SealedHookKind for InputAdapter {}

impl HookKind for InputAdapter {
    /// The step being built, and the key of the input being resolved.
    type Context = (StepName, &'static str);
}
