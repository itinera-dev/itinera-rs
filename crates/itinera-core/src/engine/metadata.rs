//! The journey's global metadata: the journey ID and the instance's own entries, set before the
//! journey starts, and the entries steps, hooks and service providers set while it runs.

use std::iter;

use super::emitter::reporter_failed;
use super::end::Aborted;
use super::{Delivery, Failures, Journey};
use crate::error::{Error, Interrupted};
use crate::event::{EventBody, JourneyAbort, Source};
use crate::journey::{Abort, JourneyId, MetadataSetter};
use crate::report::GlobalMetadata;
use crate::value::AnyValue;

/// Sets the journey ID, then each entry of the instance's initial global metadata, before the
/// journey starts. The error is that of the reporter that failed, or else of the dispatcher.
pub(crate) async fn introduce(
    delivery: &mut impl Delivery,
    failures: &Failures,
    journey_id: &JourneyId,
    metadata: GlobalMetadata,
) -> Result<(), Error> {
    let journey_id = AnyValue::new(journey_id.to_string());
    let entries = iter::once((GlobalMetadata::JOURNEY_ID.to_owned(), journey_id)).chain(metadata);
    for (key, value) in entries {
        let set = delivery.set_global_metadata(&key, &value).await;
        failures.take().map_or(set, Err)?;
    }
    Ok(())
}

impl<D: Delivery> Journey<D> {
    /// Sets an entry of global metadata, then emits `global_metadata_set`. Its key may not be
    /// the journey ID's own, and a reporter that fails on it fails the journey as on an event.
    async fn set_global_metadata(
        &mut self,
        source: Source,
        key: String,
        value: AnyValue,
    ) -> Result<(), Box<Aborted>> {
        if key == GlobalMetadata::JOURNEY_ID {
            return Err(reserved_metadata_key(source));
        }
        let delivered = self.delivery.set_global_metadata(&key, &value).await;
        if let Err(error) = self.failures.take().map_or(delivered, Err) {
            return Err(reporter_failed(self.step, error));
        }
        self.emit(EventBody::GlobalMetadataSet { key, source })
            .await
    }

    /// Sets an entry of global metadata for a step or hook, unless a reporter failed while it
    /// ran. A failure now is recorded, and interrupts it.
    pub(super) async fn set_interruptibly(
        &mut self,
        source: Source,
        key: String,
        value: AnyValue,
    ) -> Result<(), Interrupted> {
        if self.interrupted.is_some() {
            return Err(Interrupted);
        }
        let set = self.set_global_metadata(source, key, value).await;
        self.interrupt_on(set)
    }
}

fn reserved_metadata_key(source: Source) -> Box<Aborted> {
    Box::new(Aborted {
        abort: Abort::ReservedMetadataKey(MetadataSetter::from(&source)),
        reported: JourneyAbort::ReservedMetadataKey { source },
    })
}

#[cfg(test)]
mod tests;
