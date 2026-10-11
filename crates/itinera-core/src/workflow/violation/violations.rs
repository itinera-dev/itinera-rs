use super::Violation;

/// Every violation found in a workflow's declaration, at least one, which kept it from being
/// built.
///
/// # Examples
///
/// ```
/// use itinera::workflow::Violations;
///
/// fn report(violations: &Violations) -> Vec<String> {
///     violations.iter().map(ToString::to_string).collect()
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error, derive_more::IntoIterator)]
#[error("the workflow's declaration has violations: {}", listed(.violations))]
#[into_iterator(owned, ref)]
pub struct Violations {
    pub(super) violations: Vec<Violation>,
}

impl Violations {
    /// The violations, in a fixed order.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::workflow::{Violation, Violations};
    ///
    /// fn first(violations: &Violations) -> Option<&Violation> {
    ///     violations.iter().next()
    /// }
    /// ```
    pub fn iter(&self) -> std::slice::Iter<'_, Violation> {
        self.violations.iter()
    }
}

fn listed(violations: &[Violation]) -> String {
    violations
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use crate::workflow::fixtures::{adapter, orders, step};

    #[test]
    fn violations_display_every_violation() {
        let builder = orders()
            .step(step("charge"))
            .step(step("charge"))
            .input_adapter(adapter("pricing", "refund"));
        assert_eq!(
            builder.build().unwrap_err().to_string(),
            "the workflow's declaration has violations: duplicate step name: charge; \
             input adapter for unknown step: pricing on refund"
        );
    }
}
