//! A journey's global metadata: entries of a key and a value that every reporter receives.

use crate::value::AnyValue;

/// Entries of a key and a value, kept apart from the data bag, that a journey's dispatcher
/// passes to every reporter as soon as each is set.
///
/// The journey ID is always the first entry, as a string under
/// [`JOURNEY_ID`](GlobalMetadata::JOURNEY_ID). A workflow instance may be created with more, and
/// steps, hooks and service providers set entries through their reporters while the journey
/// runs. Nothing in the journey can read them back: they exist for reporters, which decide what
/// to do with them. A reporter that keeps them, to use with the events that follow, can keep
/// them in a `GlobalMetadata` of its own.
///
/// Each key has one entry: inserting a key again replaces its value, where the key first was.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::event::Event;
/// use itinera::report::{GlobalMetadata, Reporter};
/// use itinera::value::AnyValue;
///
/// #[derive(Default)]
/// struct Audit {
///     metadata: GlobalMetadata,
/// }
///
/// impl Reporter for Audit {
///     fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<(), Error> {
///         self.metadata.insert(key, value.clone());
///         Ok(())
///     }
///
///     fn report(&mut self, event: &Event) -> Result<(), Error> {
///         let user = self.metadata.get("user_id").and_then(AnyValue::downcast_ref::<String>);
///         eprintln!("{} for {user:?}", event.kind());
///         Ok(())
///     }
/// }
/// ```
#[derive(Clone, Debug, Default, derive_more::IntoIterator)]
#[into_iterator(owned, ref)]
pub struct GlobalMetadata {
    entries: Vec<(String, AnyValue)>,
}

impl GlobalMetadata {
    /// The key of the journey ID, which only the executor sets.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::report::GlobalMetadata;
    ///
    /// assert_eq!(GlobalMetadata::JOURNEY_ID, "journey_id");
    /// ```
    pub const JOURNEY_ID: &'static str = "journey_id";

    /// Global metadata with no entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::report::GlobalMetadata;
    ///
    /// assert!(GlobalMetadata::new().get("user_id").is_none());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the entry of `key` to `value`, replacing the value it had, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::report::GlobalMetadata;
    /// use itinera::value::AnyValue;
    ///
    /// let mut metadata = GlobalMetadata::new();
    /// metadata.insert("user_id", AnyValue::new("u-7".to_string()));
    /// metadata.insert("user_id", AnyValue::new("u-8".to_string()));
    /// let user = metadata.get("user_id").and_then(AnyValue::downcast_ref::<String>);
    /// assert_eq!(user.map(String::as_str), Some("u-8"));
    /// ```
    pub fn insert(&mut self, key: impl Into<String>, value: AnyValue) {
        let key = key.into();
        match self
            .entries
            .iter_mut()
            .find(|entry| is_entry_of(entry, &key))
        {
            Some((_, held)) => *held = value,
            None => self.entries.push((key, value)),
        }
    }

    /// The value of `key`, if it has an entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::report::GlobalMetadata;
    /// use itinera::value::AnyValue;
    ///
    /// let mut metadata = GlobalMetadata::new();
    /// metadata.insert("tenant", AnyValue::new("acme".to_string()));
    /// assert!(metadata.get("tenant").is_some());
    /// assert!(metadata.get("region").is_none());
    /// ```
    pub fn get(&self, key: &str) -> Option<&AnyValue> {
        self.entries
            .iter()
            .find(|entry| is_entry_of(entry, key))
            .map(|(_, value)| value)
    }

    /// Whether it sets the journey ID's own key, which only the executor may set.
    pub(crate) fn sets_journey_id(&self) -> bool {
        self.get(Self::JOURNEY_ID).is_some()
    }
}

fn is_entry_of((held, _): &(String, AnyValue), key: &str) -> bool {
    held == key
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(metadata: &GlobalMetadata) -> Vec<&str> {
        metadata.into_iter().map(|(key, _)| key.as_str()).collect()
    }

    #[test]
    fn a_key_inserted_again_keeps_its_place_and_takes_the_new_value() {
        let mut metadata = GlobalMetadata::new();
        metadata.insert("tenant", AnyValue::new("acme".to_string()));
        metadata.insert("region", AnyValue::new("eu-west".to_string()));
        metadata.insert("tenant", AnyValue::new("globex".to_string()));

        assert_eq!(keys(&metadata), ["tenant", "region"]);
        assert_eq!(
            metadata
                .get("tenant")
                .and_then(AnyValue::downcast_ref::<String>),
            Some(&"globex".to_string())
        );
    }
}
