//! Resolving a step's inputs: from its input adapter, if it has one that supplies the input, and
//! otherwise from the data bag.

use super::attempt::could_not_build;
use super::end::End;
use super::request::{Requesting, wrong_type};
use super::{Delivery, Journey};
use crate::event::EventBody;
use crate::journey::{DataBag, DataBagAccess, JourneyId};
use crate::policy::{Answers, Requested};
use crate::step::{InputNeed, StepAttempt};
use crate::value::AnyValue;
use crate::workflow::InputAdapterDescriptor;

/// What a step's inputs are resolved from: its input adapter, if it has one, and the journey's
/// data.
pub(super) struct Sources<'i, W> {
    pub(super) adapter: Option<&'i InputAdapterDescriptor<W>>,
    pub(super) workflow: &'i W,
    pub(super) journey_id: &'i JourneyId,
    pub(super) data_bag: &'i DataBag,
}

impl<D: Delivery> Journey<D> {
    /// Resolves one input of a step being built: from the step's input adapter, if it has one
    /// that supplies the input, and otherwise from the data bag.
    pub(super) async fn input<W>(
        &mut self,
        sources: &Sources<'_, W>,
        attempt: &StepAttempt,
        input: &InputNeed,
    ) -> Result<Option<AnyValue>, End> {
        let adapted = match sources.adapter {
            Some(adapter) => self.adapt(sources, adapter, attempt, input).await?,
            None => None,
        };
        match adapted {
            Some(value) => Ok(Some(value)),
            None => {
                let found = sources.data_bag.get(input.key()).cloned();
                self.request(&Requesting::step(attempt), input, found).await
            }
        }
    }

    /// Calls the step's input adapter for one of its inputs, once what the adapter requests is
    /// resolved: the value it supplied, or `None` when it does not supply this input.
    async fn adapt<W>(
        &mut self,
        sources: &Sources<'_, W>,
        adapter: &InputAdapterDescriptor<W>,
        attempt: &StepAttempt,
        input: &InputNeed,
    ) -> Result<Option<AnyValue>, End> {
        let name = adapter.name();
        let key = input.key();
        let requesting = Requesting::adapter(name, attempt);
        let data_bag = sources.data_bag;
        let mut answers = Answers::default();
        let needs = adapter.needs();
        for request in needs.requests() {
            self.answer(&mut answers, &requesting, request, data_bag, None)
                .await?;
        }
        if needs.wants_data_bag() {
            answers.data_bag = Some(DataBagAccess::new(data_bag));
        }
        let got = Requested::new(
            sources.workflow,
            sources.journey_id,
            (attempt.step, key),
            answers,
        );
        match adapter.adapt(sources.workflow, got) {
            Ok(Some(value)) => {
                self.emit(EventBody::InputAdapterSupplied {
                    step: attempt.clone(),
                    key: key.to_owned(),
                    adapter: name,
                })
                .await?;
                if input.accepts(&value) {
                    Ok(Some(value))
                } else {
                    Err(wrong_type(key, &requesting))
                }
            }
            Ok(None) => Ok(None),
            Err(error) => {
                self.emit(EventBody::InputAdapterFailed {
                    step: attempt.clone(),
                    key: key.to_owned(),
                    adapter: name,
                })
                .await?;
                Err(could_not_build(attempt.step, error))
            }
        }
    }
}

#[cfg(test)]
mod tests;
