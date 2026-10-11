//! What a hook declares it needs, and what it receives when it is called.

use crate::error::Error;
use crate::journey::{Contributor, DataBagAccess};
use crate::step::{InputNeed, Reason, Reporting, Requirement, Slots};

mod hook_needs;
mod requested;

pub use hook_needs::HookNeeds;
pub use requested::Requested;

/// What a hook needs, whatever its kind.
#[derive(Clone, Debug, Default)]
pub(crate) struct Needs {
    requests: Vec<Request>,
    data_bag: bool,
    contributor: bool,
    reporter: bool,
}

/// One thing a hook requests from the executor.
#[derive(Clone, Debug)]
pub(crate) enum Request {
    FromStep(InputNeed),
    FromWorkflow(InputNeed),
    Reason(Requirement),
    Error(Requirement),
}

impl Needs {
    /// What the hook requests, in the order it declared it.
    pub(crate) fn requests(&self) -> &[Request] {
        &self.requests
    }

    pub(crate) fn wants_data_bag(&self) -> bool {
        self.data_bag
    }

    pub(crate) fn wants_contributor(&self) -> bool {
        self.contributor
    }

    pub(crate) fn wants_reporter(&self) -> bool {
        self.reporter
    }

    fn request(&mut self, request: Request) {
        self.requests.push(request);
    }

    /// Requests the reason, replacing an earlier request for it.
    fn request_reason(&mut self, requirement: Requirement) {
        self.requests.retain(other_than_reason);
        self.request(Request::Reason(requirement));
    }

    /// Requests the error, replacing an earlier request for it.
    fn request_error(&mut self, requirement: Requirement) {
        self.requests.retain(other_than_error);
        self.request(Request::Error(requirement));
    }
}

fn other_than_reason(request: &Request) -> bool {
    !matches!(request, Request::Reason(_))
}

fn other_than_error(request: &Request) -> bool {
    !matches!(request, Request::Error(_))
}

/// The values the executor resolved for what a hook declared, and the handles it declared.
#[derive(Debug, Default)]
pub(crate) struct Answers<'a> {
    pub(crate) from_step: Slots,
    pub(crate) from_workflow: Slots,
    /// The reason, if declared: present or absent.
    pub(crate) reason: Option<Option<Reason>>,
    /// The error, if declared: present or absent.
    pub(crate) error: Option<Option<&'a Error>>,
    pub(crate) data_bag: Option<DataBagAccess<'a>>,
    pub(crate) contributor: Option<Contributor<'a>>,
    pub(crate) reporting: Option<Reporting<'a>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::fixtures::{crashed, travel_needing};
    use crate::journey::JourneyStatus;

    #[test]
    fn a_reason_declared_again_replaces_the_earlier_declaration() {
        let (status, _) = travel_needing(HookNeeds::new().reason().optional_reason(), crashed);

        assert!(matches!(status, JourneyStatus::Failed { .. }));
    }
}
