use crate::value::{AnyValue, Value};

/// A step's or hook's means of contributing data to the journey: values under keys, which are not
/// declared in advance.
///
/// It is made for one attempt and borrows it, so it cannot outlive it. Each value is captured when
/// it is contributed, and a key contributed twice in one attempt keeps its last value. A step's
/// contributions reach the data bag only when its attempt succeeds, in the order their keys were
/// first contributed.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::journey::Contributor;
/// use itinera::step::{Outcome, Resolved, Step, StepFactory, StepNeeds};
///
/// struct Receipt<'a> {
///     contributor: Contributor<'a>,
/// }
///
/// impl Step for Receipt<'_> {
///     fn run(mut self) -> Result<Outcome, Error> {
///         self.contributor.contribute("receipt", "R-1".to_string());
///         Ok(Outcome::success())
///     }
/// }
///
/// struct ReceiptFactory;
///
/// impl StepFactory for ReceiptFactory {
///     type Step<'a> = Receipt<'a>;
///
///     fn needs(&self) -> StepNeeds {
///         StepNeeds::new().contributor()
///     }
///
///     fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Receipt<'a>, Error> {
///         Ok(Receipt {
///             contributor: got.contributor()?,
///         })
///     }
/// }
/// ```
#[derive(Debug)]
pub struct Contributor<'a> {
    contributions: &'a mut Contributions,
}

impl<'a> Contributor<'a> {
    pub(crate) fn new(contributions: &'a mut Contributions) -> Self {
        Self { contributions }
    }

    /// Contributes a value under a key, replacing what this attempt contributed under it before.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::Contributor;
    ///
    /// fn issue_receipt(contributor: &mut Contributor<'_>) {
    ///     contributor.contribute("receipt", "R-1".to_string());
    /// }
    /// ```
    pub fn contribute<T: Value>(&mut self, key: impl Into<String>, value: T) {
        self.contributions.add(key.into(), AnyValue::new(value));
    }
}

/// What one attempt contributed: each key with its last value, in the order the keys were first
/// contributed.
#[derive(Debug, Default, derive_more::IntoIterator)]
pub(crate) struct Contributions {
    contributions: Vec<Contribution>,
}

impl Contributions {
    fn add(&mut self, key: String, value: AnyValue) {
        match self
            .contributions
            .iter_mut()
            .find(|earlier| earlier.is(&key))
        {
            Some(earlier) => earlier.value = value,
            None => self.contributions.push(Contribution { key, value }),
        }
    }
}

/// A value contributed under a key.
#[derive(Debug)]
pub(crate) struct Contribution {
    pub(crate) key: String,
    pub(crate) value: AnyValue,
}

impl Contribution {
    fn is(&self, key: &str) -> bool {
        self.key == key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys_and_numbers(contributions: Contributions) -> Vec<(String, i64)> {
        contributions.into_iter().map(key_and_number).collect()
    }

    fn key_and_number(contribution: Contribution) -> (String, i64) {
        let number = contribution.value.downcast_ref::<i64>().copied().unwrap();
        (contribution.key, number)
    }

    #[test]
    fn a_key_contributed_twice_keeps_its_last_value_in_its_first_place() {
        let mut contributions = Contributions::default();
        let mut contributor = Contributor::new(&mut contributions);

        contributor.contribute("amount", 1_i64);
        contributor.contribute("fee", 2_i64);
        contributor.contribute("amount", 3_i64);

        assert_eq!(
            keys_and_numbers(contributions),
            [("amount".to_string(), 3), ("fee".to_string(), 2)]
        );
    }

    #[test]
    fn a_value_is_captured_when_it_is_contributed() {
        let mut contributions = Contributions::default();
        let mut contributor = Contributor::new(&mut contributions);
        let mut tags = vec!["new".to_string()];

        contributor.contribute("tags", tags.clone());
        tags.push("late".to_string());

        let contribution = contributions.into_iter().next().unwrap();
        assert_eq!(
            contribution.value.downcast_ref::<Vec<String>>(),
            Some(&vec!["new".to_string()])
        );
    }
}
