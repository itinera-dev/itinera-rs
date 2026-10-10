//! The executors a scenario runs under.

/// One of itinera's executors. A scenario runs once under each executor that accepts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Executor {
    Local,
    AsyncLocal,
}

impl Executor {
    const ALL: [Self; 2] = [Self::Local, Self::AsyncLocal];

    /// The tag the runner gives the copy of a scenario that runs under this executor.
    pub(crate) fn tag(self) -> &'static str {
        match self {
            Self::Local => "executor-local",
            Self::AsyncLocal => "executor-async",
        }
    }

    /// The executors that run a scenario with these tags: only the one of the execution mode it
    /// needs, if it needs one, and both otherwise.
    pub(crate) fn running<S: AsRef<str>>(tags: impl IntoIterator<Item = S>) -> &'static [Self] {
        tags.into_iter()
            .find_map(|tag| Self::required_by(tag.as_ref()))
            .unwrap_or(&Self::ALL)
    }

    /// The executor a copy of a scenario with these tags runs under, from the tag the runner
    /// gave the copy.
    pub(crate) fn tagged<S: AsRef<str>>(tags: impl IntoIterator<Item = S>) -> Option<Self> {
        tags.into_iter()
            .find_map(|tag| Self::with_tag(tag.as_ref()))
    }

    fn with_tag(tag: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|executor| executor.is_tagged(tag))
    }

    fn is_tagged(self, tag: &str) -> bool {
        self.tag() == tag
    }

    /// The executor a capability tag requires, if the tag names an execution mode.
    fn required_by(tag: &str) -> Option<&'static [Self]> {
        match tag {
            "capability-sync" => Some(&[Self::Local]),
            "capability-async" => Some(&[Self::AsyncLocal]),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::local(&["tier-1", "executor-local"], Some(Executor::Local))]
    #[case::async_local(&["executor-async", "capability-async"], Some(Executor::AsyncLocal))]
    #[case::untagged(&["tier-1"], None)]
    fn a_copy_runs_under_the_executor_its_tag_names(
        #[case] tags: &[&str],
        #[case] executor: Option<Executor>,
    ) {
        assert_eq!(Executor::tagged(tags), executor);
    }
}
