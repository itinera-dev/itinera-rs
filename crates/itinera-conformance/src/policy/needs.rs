//! Declaring what a hook requests, of the Rust type of each request's type.

use itinera::policy::{ErrorHookKind, FailureHookKind, HookKind, HookNeeds, StepHookKind};
use itinera::step::{Input, OptionalInput};
use itinera::value::Value as Storable;

use super::request::Request;
use crate::model::ValueType;
use crate::value::{Data, ForType, for_type};

/// Declares a request every hook called because an attempt did not succeed may make.
pub(super) fn declare_for_failure<H: FailureHookKind + ErrorHookKind>(
    needs: HookNeeds<H>,
    request: &Request,
) -> Result<HookNeeds<H>, String> {
    match request {
        Request::Reason { optional: false } => Ok(needs.reason()),
        Request::Reason { optional: true } => Ok(needs.optional_reason()),
        _ => declare_for_error(needs, request),
    }
}

/// Declares a request every hook that may request the error may make.
pub(super) fn declare_for_error<H: ErrorHookKind>(
    needs: HookNeeds<H>,
    request: &Request,
) -> Result<HookNeeds<H>, String> {
    match request {
        Request::Error => Ok(needs.error()),
        _ => declare_for_step(needs, request),
    }
}

/// Declares a request every step hook may make.
pub(super) fn declare_for_step<H: StepHookKind>(
    needs: HookNeeds<H>,
    request: &Request,
) -> Result<HookNeeds<H>, String> {
    match request {
        Request::StepData(data) => Ok(for_type(data.value_type, NeedingFromStep { needs, data })),
        Request::StepName | Request::Attempt => Ok(needs),
        _ => declare_for_any(needs, request),
    }
}

/// Declares a request every hook may make.
pub(super) fn declare_for_any<H: HookKind>(
    needs: HookNeeds<H>,
    request: &Request,
) -> Result<HookNeeds<H>, String> {
    match request {
        Request::WorkflowData(data) => Ok(for_type(
            data.value_type,
            NeedingFromWorkflow { needs, data },
        )),
        Request::JourneyId => Ok(needs),
        _ => Err(request.to_string()),
    }
}

/// Declares data from the step, of the Rust type of its type.
struct NeedingFromStep<'d, H> {
    needs: HookNeeds<H>,
    data: &'d Data,
}

impl<H: StepHookKind> ForType for NeedingFromStep<'_, H> {
    type Output = HookNeeds<H>;

    fn of<T: Storable>(self) -> HookNeeds<H> {
        if self.data.optional {
            self.needs
                .optional_from_step(&OptionalInput::<T>::new(self.data.key))
        } else {
            self.needs.from_step(&Input::<T>::new(self.data.key))
        }
    }
}

/// Declares data from the workflow, of the Rust type of its type.
struct NeedingFromWorkflow<'d, H> {
    needs: HookNeeds<H>,
    data: &'d Data,
}

impl<H: HookKind> ForType for NeedingFromWorkflow<'_, H> {
    type Output = HookNeeds<H>;

    fn of<T: Storable>(self) -> HookNeeds<H> {
        if self.data.optional {
            self.needs
                .optional_from_workflow(&OptionalInput::<T>::new(self.data.key))
        } else {
            self.needs.from_workflow(&Input::<T>::new(self.data.key))
        }
    }
}

/// Declares required data from the workflow under the key, of the Rust type of its type.
pub(crate) fn requiring_from_workflow<H: HookKind>(
    needs: HookNeeds<H>,
    (key, value_type): &(String, ValueType),
) -> HookNeeds<H> {
    let data = Data::of(key, *value_type, false);
    for_type(*value_type, NeedingFromWorkflow { needs, data: &data })
}

#[cfg(test)]
mod tests {
    use itinera::policy::{StepHook, WorkflowHook};
    use rstest::rstest;

    use super::*;
    use crate::model::{self, Hook, HookRequest, ModelError};
    use crate::policy::Scripts;

    fn requesting(hook: Hook, request: HookRequest) -> Result<Scripts, ModelError> {
        let mut policy = model::Policy::new(hook);
        policy.hook_mut(hook).unwrap().requests.push(request);
        Scripts::of("policy", &policy)
    }

    #[rstest]
    #[case::step_data_from_a_workflow_hook(
        Hook::Workflow(WorkflowHook::OnWorkflowSuccess),
        HookRequest::StepData {
            key: "receipt".to_owned(),
            value_type: ValueType::String,
            optional: false,
        },
        "step data"
    )]
    #[case::the_retry_cause_from_on_step_failure(
        Hook::Step(StepHook::OnStepFailure),
        HookRequest::RetryCause,
        "the retry cause"
    )]
    #[case::the_failure_reason_from_on_step_abnormal_termination(
        Hook::Step(StepHook::OnStepAbnormalTermination),
        HookRequest::FailureReason { optional: false },
        "the failure reason"
    )]
    fn a_request_a_hook_cannot_make_is_a_case_error(
        #[case] hook: Hook,
        #[case] request: HookRequest,
        #[case] named: &str,
    ) {
        assert_eq!(
            requesting(hook, request).unwrap_err(),
            ModelError::RequestNotAllowed(hook, named.to_owned())
        );
    }
}
