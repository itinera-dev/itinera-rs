//! A step's inputs, required or optional: a key and the type its value must have.

use std::marker::PhantomData;

use crate::value::Value;

/// A required input of a step: a key, local to the step, and the type its value must have.
///
/// A step factory keeps its inputs, declares them in its [`StepNeeds`], and takes their values
/// from [`Resolved`] when it builds the step.
///
/// [`StepNeeds`]: super::StepNeeds
/// [`Resolved`]: super::Resolved
///
/// # Examples
///
/// ```
/// use itinera::step::Input;
///
/// const AMOUNT: Input<i64> = Input::new("amount");
/// assert_eq!(AMOUNT.key(), "amount");
/// ```
#[derive(derive_more::Debug)]
pub struct Input<T> {
    pub(super) key: &'static str,
    #[debug(skip)]
    value: PhantomData<fn() -> T>,
}

impl<T: Value> Input<T> {
    /// Declares a required input under this key.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::Input;
    ///
    /// let customer = Input::<String>::new("customer");
    /// ```
    pub const fn new(key: &'static str) -> Self {
        Self {
            key,
            value: PhantomData,
        }
    }

    /// The input's key.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::Input;
    ///
    /// assert_eq!(Input::<bool>::new("express").key(), "express");
    /// ```
    pub fn key(&self) -> &'static str {
        self.key
    }
}

/// An optional input of a step: a key, local to the step, and the type its value must have if
/// there is one.
///
/// # Examples
///
/// ```
/// use itinera::step::OptionalInput;
///
/// const DISCOUNT: OptionalInput<i64> = OptionalInput::new("discount");
/// assert_eq!(DISCOUNT.key(), "discount");
/// ```
#[derive(derive_more::Debug)]
pub struct OptionalInput<T> {
    pub(super) key: &'static str,
    #[debug(skip)]
    value: PhantomData<fn() -> T>,
}

impl<T: Value> OptionalInput<T> {
    /// Declares an optional input under this key.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::OptionalInput;
    ///
    /// let note = OptionalInput::<String>::new("note");
    /// ```
    pub const fn new(key: &'static str) -> Self {
        Self {
            key,
            value: PhantomData,
        }
    }

    /// The input's key.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::OptionalInput;
    ///
    /// assert_eq!(OptionalInput::<bool>::new("gift").key(), "gift");
    /// ```
    pub fn key(&self) -> &'static str {
        self.key
    }
}
