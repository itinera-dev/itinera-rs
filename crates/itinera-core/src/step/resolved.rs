//! What the executor resolved for one attempt of a step, from which its factory builds it.

use std::any;
use std::marker::PhantomData;

use super::{Input, OptionalInput, Reporting, Slots, StepReporter};
use crate::error::Error;
use crate::journey::Contributor;
use crate::mode::Synchronous;
use crate::value::{AnyValue, Value};

/// What the executor resolved for one attempt of a step, from which its factory builds it.
///
/// Each value is the step's own: it is taken out, never borrowed from the data bag, so nothing
/// the step does to it reaches the data bag or anyone else. `M` is the workflow's mode.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::step::{Input, Resolved};
///
/// const AMOUNT: Input<i64> = Input::new("amount");
///
/// fn amount(got: &mut Resolved<'_>) -> Result<i64, Error> {
///     got.input(&AMOUNT)
/// }
/// ```
#[derive(derive_more::Debug)]
pub struct Resolved<'a, M = Synchronous> {
    got: Got<'a>,
    #[debug(skip)]
    mode: PhantomData<M>,
}

impl<'a, M> Resolved<'a, M> {
    pub(crate) fn new(got: Got<'a>) -> Self {
        Self {
            got,
            mode: PhantomData,
        }
    }

    /// Takes the value of a required input.
    ///
    /// # Errors
    ///
    /// When the step's needs do not declare this input with this type, or its value was already
    /// taken. Building fails with that error, and the journey is aborted with
    /// `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::step::{Input, Resolved};
    ///
    /// fn customer(got: &mut Resolved<'_>) -> Result<String, Error> {
    ///     got.input(&Input::new("customer"))
    /// }
    /// ```
    pub fn input<T: Value>(&mut self, input: &Input<T>) -> Result<T, Error> {
        self.got
            .take(input.key)
            .and_then(AnyValue::downcast)
            .ok_or_else(|| undeclared::<T>(input.key))
    }

    /// Takes the value of an optional input: `None` when there was none.
    ///
    /// # Errors
    ///
    /// When the step's needs do not declare this input with this type, or it was already taken.
    /// Building fails with that error, and the journey is aborted with `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::step::{OptionalInput, Resolved};
    ///
    /// fn discount(got: &mut Resolved<'_>) -> Result<i64, Error> {
    ///     Ok(got.optional_input(&OptionalInput::new("discount"))?.unwrap_or(0))
    /// }
    /// ```
    pub fn optional_input<T: Value>(
        &mut self,
        input: &OptionalInput<T>,
    ) -> Result<Option<T>, Error> {
        match self.got.take_optional(input.key) {
            Some(None) => Ok(None),
            Some(Some(value)) => value
                .downcast()
                .map(Some)
                .ok_or_else(|| undeclared::<T>(input.key)),
            None => Err(undeclared::<T>(input.key)),
        }
    }

    /// Takes the step's contributor.
    ///
    /// # Errors
    ///
    /// When the step's needs do not declare a contributor, or it was already taken. Building
    /// fails with that error, and the journey is aborted with `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::journey::Contributor;
    /// use itinera::step::Resolved;
    ///
    /// fn contributor<'a>(got: &mut Resolved<'a>) -> Result<Contributor<'a>, Error> {
    ///     got.contributor()
    /// }
    /// ```
    pub fn contributor(&mut self) -> Result<Contributor<'a>, Error> {
        self.got
            .contributor
            .take()
            .ok_or_else(|| undeclared_handle("a contributor"))
    }

    pub(crate) fn reporting(&mut self) -> Result<Reporting<'a>, Error> {
        self.got
            .reporting
            .take()
            .ok_or_else(|| undeclared_handle("a step reporter"))
    }
}

impl<'a> Resolved<'a, Synchronous> {
    /// Takes the step's reporter.
    ///
    /// # Errors
    ///
    /// When the step's needs do not declare a reporter, or it was already taken. Building fails
    /// with that error, and the journey is aborted with `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::step::{Resolved, StepReporter};
    ///
    /// fn reporter<'a>(got: &mut Resolved<'a>) -> Result<StepReporter<'a>, Error> {
    ///     got.reporter()
    /// }
    /// ```
    pub fn reporter(&mut self) -> Result<StepReporter<'a>, Error> {
        self.reporting().map(StepReporter::from)
    }
}

fn undeclared_handle(handle: &str) -> Error {
    Error::msg(format!(
        "the step does not declare {handle}, or took it already"
    ))
}

fn undeclared<T>(key: &str) -> Error {
    Error::msg(format!(
        "the step does not declare the input \"{key}\" of type {}, or took it already",
        any::type_name::<T>()
    ))
}

/// What the engine resolved for one attempt, whatever the workflow's mode.
#[derive(derive_more::Debug)]
pub(crate) struct Got<'a> {
    inputs: Slots,
    contributor: Option<Contributor<'a>>,
    reporting: Option<Reporting<'a>>,
}

impl<'a> Got<'a> {
    /// Holds the values of the inputs, in the order the needs declare them, absent ones as
    /// `None`, and the handles the needs declare.
    pub(crate) fn new(
        inputs: impl IntoIterator<Item = (&'static str, Option<AnyValue>)>,
        contributor: Option<Contributor<'a>>,
        reporting: Option<Reporting<'a>>,
    ) -> Self {
        Self {
            inputs: inputs.into_iter().collect(),
            contributor,
            reporting,
        }
    }

    fn take(&mut self, key: &str) -> Option<AnyValue> {
        self.inputs.take(key)
    }

    fn take_optional(&mut self, key: &str) -> Option<Option<AnyValue>> {
        self.inputs.take_optional(key)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    const AMOUNT: Input<i64> = Input::new("amount");

    fn resolved<'a>(inputs: Vec<(&'static str, Option<AnyValue>)>) -> Resolved<'a> {
        Resolved::new(Got::new(inputs, None, None))
    }

    #[test]
    fn an_input_is_taken_only_once() {
        let mut got = resolved(vec![("amount", Some(AnyValue::new(42_i64)))]);

        assert_eq!(got.input(&AMOUNT).unwrap(), 42);
        assert_eq!(
            got.input(&AMOUNT).unwrap_err().to_string(),
            "the step does not declare the input \"amount\" of type i64, or took it already"
        );
    }

    type Take = fn(&mut Resolved<'_>) -> Result<(), Error>;

    fn read_amount(got: &mut Resolved<'_>) -> Result<(), Error> {
        got.input(&AMOUNT).map(drop)
    }

    fn read_discount(got: &mut Resolved<'_>) -> Result<(), Error> {
        got.optional_input(&OptionalInput::<i64>::new("discount"))
            .map(drop)
    }

    #[rstest]
    #[case::a_required_input(read_amount)]
    #[case::an_optional_input(read_discount)]
    fn an_input_the_step_does_not_declare_cannot_be_read(#[case] take: Take) {
        assert!(take(&mut resolved(Vec::new())).is_err());
    }

    #[test]
    fn an_input_read_as_another_type_than_declared_cannot_be_read() {
        let mut got = resolved(vec![("amount", Some(AnyValue::new(42_i64)))]);

        assert!(got.input(&Input::<i32>::new("amount")).is_err());
    }

    fn take_contributor(got: &mut Resolved<'_>) -> Result<(), Error> {
        got.contributor().map(drop)
    }

    fn take_reporter(got: &mut Resolved<'_>) -> Result<(), Error> {
        got.reporter().map(drop)
    }

    #[rstest]
    #[case::a_contributor(take_contributor, "a contributor")]
    #[case::a_step_reporter(take_reporter, "a step reporter")]
    fn a_handle_the_step_does_not_declare_cannot_be_taken(
        #[case] take: Take,
        #[case] handle: &str,
    ) {
        let error = take(&mut resolved(Vec::new())).unwrap_err();

        assert_eq!(
            error.to_string(),
            format!("the step does not declare {handle}, or took it already")
        );
    }
}
