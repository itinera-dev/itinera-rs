//! The executors a scenario runs under.

/// One of itinera's executors. A scenario runs once under each executor that accepts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Executor {
    Local,
    AsyncLocal,
}

impl Executor {
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
            .unwrap_or(&[Self::Local, Self::AsyncLocal])
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
