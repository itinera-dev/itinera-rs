use crate::value::{AnyValue, Value};

/// A step's or hook's means of contributing data to the journey: values under keys, which are not
/// declared in advance.
///
/// itinera keeps one for each attempt, and for each call of a hook, and lends it as
/// `&mut Contributor`, so a step or hook cannot keep it beyond that attempt or call. Outside the
/// `testing` feature only itinera makes one, so a step cannot replace what it lends; with it, a
/// contributor implements `Default`, for the recorder `Contributions`. Each value is captured when
/// it is contributed, and a key contributed twice in one attempt keeps its last value. A step's
/// contributions reach the data bag only when its attempt succeeds, in the order their keys were
/// first contributed.
///
/// # Examples
///
/// ```
/// use itinera::error::Result;
/// use itinera::journey::Contributor;
/// use itinera::step::{Outcome, Resolved, Step, StepFactory, StepNeeds};
///
/// struct Receipt<'a> {
///     contributor: &'a mut Contributor,
/// }
///
/// impl Step for Receipt<'_> {
///     fn run(self) -> Result<Outcome> {
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
///     fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Receipt<'a>> {
///         Ok(Receipt {
///             contributor: got.contributor()?,
///         })
///     }
/// }
/// ```
#[derive(Debug)]
#[cfg_attr(any(test, feature = "testing"), derive(Default))]
pub struct Contributor {
    contributions: Vec<Contribution>,
}

impl Contributor {
    pub(crate) fn new() -> Self {
        Self {
            contributions: Vec::new(),
        }
    }

    /// A copy of what was contributed, for the engine alone: a public `Clone` would let a step
    /// take back what it contributed.
    pub(crate) fn copied(&self) -> Self {
        Self {
            contributions: self.contributions.clone(),
        }
    }

    /// Contributes a value under a key, replacing what this attempt contributed under it before.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::Contributor;
    ///
    /// fn issue_receipt(contributor: &mut Contributor) {
    ///     contributor.contribute("receipt", "R-1".to_string());
    /// }
    /// ```
    pub fn contribute<T: Value>(&mut self, key: impl Into<String>, value: T) {
        self.add(key.into(), AnyValue::new(value));
    }

    /// Each key with its last value, in the order the keys were first contributed.
    pub(crate) fn iter(&self) -> std::slice::Iter<'_, Contribution> {
        self.contributions.iter()
    }

    /// The value contributed under the key, if any.
    pub(crate) fn get(&self, key: &str) -> Option<&AnyValue> {
        self.contributions
            .iter()
            .find(|contribution| contribution.is(key))
            .map(|contribution| &contribution.value)
    }

    /// The keys contributed, in the order they were first contributed.
    #[cfg(any(test, feature = "testing"))]
    pub(crate) fn keys(&self) -> impl Iterator<Item = &str> {
        self.contributions.iter().map(Contribution::key)
    }

    pub(crate) fn add(&mut self, key: String, value: AnyValue) {
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
#[derive(Clone, Debug)]
pub(crate) struct Contribution {
    pub(crate) key: String,
    pub(crate) value: AnyValue,
}

impl Contribution {
    fn is(&self, key: &str) -> bool {
        self.key == key
    }

    #[cfg(any(test, feature = "testing"))]
    fn key(&self) -> &str {
        &self.key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys_and_numbers(contributions: Contributor) -> Vec<(String, i64)> {
        contributions
            .contributions
            .into_iter()
            .map(key_and_number)
            .collect()
    }

    fn key_and_number(contribution: Contribution) -> (String, i64) {
        let number = contribution.value.downcast_ref::<i64>().copied().unwrap();
        (contribution.key, number)
    }

    #[test]
    fn a_key_contributed_twice_keeps_its_last_value_in_its_first_place() {
        let mut contributor = Contributor::default();

        contributor.contribute("amount", 1_i64);
        contributor.contribute("fee", 2_i64);
        contributor.contribute("amount", 3_i64);

        assert_eq!(contributor.keys().collect::<Vec<_>>(), ["amount", "fee"]);
        assert_eq!(
            keys_and_numbers(contributor),
            [("amount".to_string(), 3), ("fee".to_string(), 2)]
        );
    }

    #[test]
    fn a_value_is_captured_when_it_is_contributed() {
        let mut contributor = Contributor::default();
        let mut tags = vec!["new".to_string()];

        contributor.contribute("tags", tags.clone());
        tags.push("late".to_string());

        let contribution = contributor.contributions.into_iter().next().unwrap();
        assert_eq!(
            contribution.value.downcast_ref::<Vec<String>>(),
            Some(&vec!["new".to_string()])
        );
    }
}
