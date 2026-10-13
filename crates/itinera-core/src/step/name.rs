//! Step names, including the names the executor gives rescue steps.

use std::collections::BTreeSet;
use std::sync::{Mutex, PoisonError};

/// A step's name: non-empty text, unique within its workflow and compared case-sensitively. A
/// declared step's name is fixed when the program is compiled; a rescue step's is made from the
/// name of the step it rescues, once for each such name.
///
/// [`step_name!`] makes one from a constant, and refuses an empty one at compile time.
/// [`StepName::new`] does the same from any `&'static str`, at compile time in a `const` context
/// and otherwise when it is called.
///
/// # Examples
///
/// ```
/// use itinera::step::{StepName, step_name};
///
/// let charge: StepName = step_name!("charge");
/// assert_eq!(charge.to_string(), "charge");
/// let text: &str = charge.as_ref();
/// assert_eq!(text, "charge");
/// ```
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    derive_more::Display,
    derive_more::Into,
    derive_more::AsRef,
)]
#[display("{name}")]
#[as_ref(forward)]
pub struct StepName {
    name: &'static str,
}

impl StepName {
    /// Makes a step name.
    ///
    /// # Panics
    ///
    /// Panics if the name is empty, which is a mistake in the workflow's declaration. In a
    /// `const` context, such as [`step_name!`], that is a compile error instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::StepName;
    ///
    /// const SHIP: StepName = StepName::new("ship");
    /// assert_eq!(SHIP.to_string(), "ship");
    /// ```
    #[expect(
        clippy::panic,
        reason = "an empty step name is a mistake in a declaration, before any journey"
    )]
    pub const fn new(name: &'static str) -> Self {
        if name.is_empty() {
            panic!("a step name cannot be empty");
        }
        Self { name }
    }

    /// Whether this is the name written as `name`.
    #[cfg(any(test, feature = "testing"))]
    pub(crate) fn is(self, name: &str) -> bool {
        self.name == name
    }

    /// The name of this step's rescue step: this name followed by `_rescue`.
    ///
    /// Each rescue step's name is made once for the whole program and kept, as step names are
    /// kept, so describing a workflow again takes no more memory.
    pub(crate) fn rescue(self) -> Self {
        let name = format!("{}_rescue", self.name);
        let mut made = RESCUE_NAMES.lock().unwrap_or_else(PoisonError::into_inner);
        let kept = match made.get(name.as_str()) {
            Some(kept) => kept,
            None => {
                let kept: &'static str = Box::leak(name.into_boxed_str());
                made.insert(kept);
                kept
            }
        };
        Self { name: kept }
    }
}

/// The rescue steps' names made so far.
static RESCUE_NAMES: Mutex<BTreeSet<&'static str>> = Mutex::new(BTreeSet::new());

/// Makes a [`StepName`] from a constant, and refuses an empty one at compile time.
///
/// # Examples
///
/// ```
/// use itinera::step::step_name;
///
/// assert_eq!(step_name!("charge").to_string(), "charge");
/// ```
///
/// An empty name does not compile:
///
/// ```compile_fail,E0080
/// use itinera::step::step_name;
///
/// let nameless = step_name!("");
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __step_name {
    ($name:expr) => {
        const { $crate::step::StepName::new($name) }
    };
}

#[doc(inline)]
pub use crate::__step_name as step_name;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "a step name cannot be empty")]
    fn a_step_name_cannot_be_empty() {
        let _ = StepName::new("");
    }

    #[test]
    fn a_rescue_steps_name_is_the_steps_name_followed_by_rescue() {
        assert_eq!(
            StepName::new("charge").rescue(),
            StepName::new("charge_rescue")
        );
    }

    #[test]
    fn a_rescue_steps_name_is_made_once_for_each_step_name() {
        let first: &str = StepName::new("refund").rescue().into();
        let again: &str = StepName::new("refund").rescue().into();

        assert!(std::ptr::eq(first, again));
    }
}
