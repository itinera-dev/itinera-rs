//! Slots: declared values by key, each taken at most once.

use std::mem;

use crate::value::AnyValue;

/// Declared values by key, each taken at most once.
#[derive(Debug, Default)]
pub(crate) struct Slots {
    received: Vec<Received>,
}

/// One declared value, until it is taken.
#[derive(Debug)]
struct Received {
    key: &'static str,
    value: Slot,
}

/// Where a declared value stands: not taken yet, present or absent, or taken.
#[derive(Debug)]
enum Slot {
    Untaken(Option<AnyValue>),
    Taken,
}

impl Received {
    fn is(&self, key: &str) -> bool {
        self.key == key
    }

    /// The key with its value, present or absent, unless it was already taken.
    #[cfg(any(test, feature = "testing"))]
    fn untaken(&self) -> Option<(&'static str, Option<AnyValue>)> {
        match &self.value {
            Slot::Untaken(value) => Some((self.key, value.clone())),
            Slot::Taken => None,
        }
    }

    /// Takes the value, present or absent, unless it was already taken.
    fn take(&mut self) -> Option<Option<AnyValue>> {
        match mem::replace(&mut self.value, Slot::Taken) {
            Slot::Untaken(value) => Some(value),
            Slot::Taken => None,
        }
    }
}

impl Slots {
    /// Holds a declared value, present or absent, after those already held.
    pub(crate) fn hold(&mut self, key: &'static str, value: Option<AnyValue>) {
        self.received.push(received((key, value)));
    }

    /// Takes a value, present or absent, unless it was not declared or was already taken.
    pub(crate) fn take(&mut self, key: &str) -> Option<Option<AnyValue>> {
        self.received
            .iter_mut()
            .find(|received| received.is(key))
            .and_then(Received::take)
    }

    /// Every value not taken yet, present or absent, with its key, in the order they were held.
    #[cfg(any(test, feature = "testing"))]
    pub(crate) fn untaken(&self) -> impl Iterator<Item = (&'static str, Option<AnyValue>)> {
        self.received.iter().filter_map(Received::untaken)
    }
}

impl FromIterator<(&'static str, Option<AnyValue>)> for Slots {
    fn from_iter<T: IntoIterator<Item = (&'static str, Option<AnyValue>)>>(values: T) -> Self {
        Self {
            received: values.into_iter().map(received).collect(),
        }
    }
}

fn received((key, value): (&'static str, Option<AnyValue>)) -> Received {
    Received {
        key,
        value: Slot::Untaken(value),
    }
}
