//! The fakes a test gives an instance in place of the scenario's service providers, through
//! itinera's test support.

use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use itinera::error::Error;
use itinera::instance::InstanceBuilder;
use itinera::mode::Mode;
use itinera::service::Provided;
use serde_json::Value;

use super::{Labeller, Made, Meter, Supplied};
use crate::declaration::{ScriptedWorkflow, leaked};
use crate::model::{Fake, Faking, Implementation};

/// How many times a fake was called in the journey, whether it made an instance or failed; its
/// clones share the count.
#[derive(Clone, Debug, Default)]
pub(crate) struct FakeCalls {
    count: Arc<AtomicU32>,
}

impl FakeCalls {
    pub(crate) fn count(&self) -> u32 {
        self.count.load(Ordering::SeqCst)
    }

    /// Counts one more call, giving its number, from 1.
    fn one_more(&self) -> NonZeroU32 {
        NonZeroU32::MIN.saturating_add(self.count.fetch_add(1, Ordering::SeqCst))
    }
}

/// The instance builder, with this fake in place of its provider, counting its calls in `calls`.
pub(crate) fn with_fake<M: Mode>(
    builder: InstanceBuilder<ScriptedWorkflow, M>,
    fake: &Fake,
    calls: FakeCalls,
) -> InstanceBuilder<ScriptedWorkflow, M> {
    let name = leaked(&fake.provider);
    match &fake.makes {
        Faking::Labellers {
            implementation,
            data,
        } => {
            let implementation = *implementation;
            let data = data.clone();
            builder.fake_provider::<dyn Labeller>(name, move || {
                labeller(&calls, implementation, data.as_ref())
            })
        }
        Faking::Fails(message) => {
            let message = message.clone();
            builder.fake_provider::<dyn Labeller>(name, move || failing(&calls, &message))
        }
        Faking::Meters => builder.fake_provider::<dyn Meter>(name, move || meter(&calls)),
    }
}

/// An instance of the implementation, labelled `fake <n>` for the fake's `n`th call.
fn labeller(
    calls: &FakeCalls,
    implementation: Implementation,
    data: Option<&Value>,
) -> Result<Provided<dyn Labeller>, Error> {
    Ok(<dyn Labeller>::supplied(
        made(calls),
        implementation,
        data.cloned(),
    ))
}

/// A meter, reading the number of the fake's call.
fn meter(calls: &FakeCalls) -> Result<Provided<dyn Meter>, Error> {
    Ok(<dyn Meter>::supplied(
        made(calls),
        Implementation::default(),
        None,
    ))
}

fn failing(calls: &FakeCalls, message: &str) -> Result<Provided<dyn Labeller>, Error> {
    calls.one_more();
    Err(Error::msg(message.to_owned()))
}

/// What the fake makes on its next call.
fn made(calls: &FakeCalls) -> Made {
    let number = calls.one_more();
    Made {
        label: format!("fake {number}"),
        number,
        failure: None,
    }
}
