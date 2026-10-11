//! Taking what a hook received for each of its requests.

use itinera::error::Error;
use itinera::policy::{ErrorHookKind, FailureHookKind, HookKind, Requested, StepHookKind};
use itinera::step::{Input, OptionalInput, Reason};
use itinera::value::Value as Storable;
use serde_json::Value;

use super::request::Request;
use crate::declaration::ScriptedWorkflow;
use crate::model;
use crate::value::{Data, ForType, for_type, json};
use crate::witness::Received;

/// Takes what a hook called because an attempt did not succeed received for a request every
/// such hook may make.
pub(super) fn receive_for_failure<H: FailureHookKind + ErrorHookKind, M>(
    got: &mut Requested<'_, ScriptedWorkflow, H, M>,
    request: &Request,
    received: &mut Received,
) -> Result<(), Error> {
    match request {
        Request::Reason { optional } => received.reason = Some(reason(got, *optional)?),
        _ => return receive_for_error(got, request, received),
    }
    Ok(())
}

/// Takes what a hook that may request the error received for a request every such hook may
/// make.
pub(super) fn receive_for_error<H: ErrorHookKind, M>(
    got: &mut Requested<'_, ScriptedWorkflow, H, M>,
    request: &Request,
    received: &mut Received,
) -> Result<(), Error> {
    match request {
        Request::Error => received.error = Some(got.error()?.to_string()),
        _ => return receive_for_step(got, request, received),
    }
    Ok(())
}

/// Takes what a step hook received for a request every step hook may make.
pub(super) fn receive_for_step<H: StepHookKind, M>(
    got: &mut Requested<'_, ScriptedWorkflow, H, M>,
    request: &Request,
    received: &mut Received,
) -> Result<(), Error> {
    match request {
        Request::StepData(data) => {
            let value = for_type(data.value_type, TakingFromStep { got, data })?;
            received.step_data.insert(data.key.to_owned(), value);
        }
        Request::StepName => received.step_name = Some(got.step_name().to_string()),
        Request::Attempt => received.attempt = Some(got.attempt()),
        _ => return receive_for_any(got, request, received),
    }
    Ok(())
}

/// Takes what a hook received for a request every hook may make.
pub(super) fn receive_for_any<H: HookKind, M>(
    got: &mut Requested<'_, ScriptedWorkflow, H, M>,
    request: &Request,
    received: &mut Received,
) -> Result<(), Error> {
    match request {
        Request::WorkflowData(data) => {
            let value = for_type(data.value_type, TakingFromWorkflow { got, data })?;
            received.workflow_data.insert(data.key.to_owned(), value);
        }
        Request::JourneyId => received.journey_id = Some(got.journey_id().to_string()),
        _ => {}
    }
    Ok(())
}

/// The failure reason the hook received, as the scenario writes one.
fn reason<H: FailureHookKind, M>(
    got: &mut Requested<'_, ScriptedWorkflow, H, M>,
    optional: bool,
) -> Result<Option<model::Reason>, Error> {
    let reason = if optional {
        got.optional_reason()?
    } else {
        Some(got.reason()?)
    };
    reason.as_ref().map(written).transpose()
}

fn written(reason: &Reason) -> Result<model::Reason, Error> {
    Ok(model::Reason {
        code: reason.code().to_owned(),
        message: reason.message().map(str::to_owned),
        details: reason.details().map(serde_json::to_value).transpose()?,
    })
}

/// Takes data from the step, as JSON, or `None` when it was absent.
struct TakingFromStep<'g, 'a, 'd, H: HookKind, M> {
    got: &'g mut Requested<'a, ScriptedWorkflow, H, M>,
    data: &'d Data,
}

impl<H: StepHookKind, M> ForType for TakingFromStep<'_, '_, '_, H, M> {
    type Output = Result<Option<Value>, Error>;

    fn of<T: Storable>(self) -> Self::Output {
        if self.data.optional {
            self.got
                .optional_from_step(&OptionalInput::<T>::new(self.data.key))?
                .map(json)
                .transpose()
        } else {
            json(self.got.from_step(&Input::<T>::new(self.data.key))?).map(Some)
        }
    }
}

/// Takes data from the workflow, as JSON, or `None` when it was absent.
struct TakingFromWorkflow<'g, 'a, 'd, H: HookKind, M> {
    got: &'g mut Requested<'a, ScriptedWorkflow, H, M>,
    data: &'d Data,
}

impl<H: HookKind, M> ForType for TakingFromWorkflow<'_, '_, '_, H, M> {
    type Output = Result<Option<Value>, Error>;

    fn of<T: Storable>(self) -> Self::Output {
        if self.data.optional {
            self.got
                .optional_from_workflow(&OptionalInput::<T>::new(self.data.key))?
                .map(json)
                .transpose()
        } else {
            json(self.got.from_workflow(&Input::<T>::new(self.data.key))?).map(Some)
        }
    }
}
