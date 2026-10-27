//! Journey IDs: the identifier of a journey, a string that is not empty.

use std::sync::Arc;

use nutype::nutype;

/// The identifier of one journey, unique to it: a string that is not empty. It is made when the
/// journey's instance is created, and cheap to clone, since clones share its text.
///
/// `try_from` makes one from a `&str`, a `String` or an `Arc<str>`, and `try_new` from an
/// `Arc<str>`; each refuses an empty string with [`EmptyJourneyId`]. `into_inner` gives its text
/// back as an `Arc<str>`.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
///
/// use itinera::journey::JourneyId;
///
/// let id = JourneyId::try_from("order-42")?;
/// let text: &str = id.as_ref();
/// assert_eq!(text, "order-42");
/// assert_eq!(id.to_string(), "order-42");
/// assert_eq!(JourneyId::try_from(String::from("order-42"))?, id);
/// assert_eq!(JourneyId::try_new(Arc::from("order-42"))?, id);
/// assert!(JourneyId::try_from("").is_err());
/// assert!(JourneyId::try_from(Arc::<str>::from("")).is_err());
/// assert_eq!(&*id.into_inner(), "order-42");
/// # Ok::<(), itinera::journey::EmptyJourneyId>(())
/// ```
#[nutype(
    validate(with = not_empty, error = EmptyJourneyId),
    derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Display, AsRef, TryFrom)
)]
pub struct JourneyId(Arc<str>);

fn not_empty(text: &Arc<str>) -> Result<(), EmptyJourneyId> {
    if text.is_empty() {
        Err(EmptyJourneyId)
    } else {
        Ok(())
    }
}

impl AsRef<str> for JourneyId {
    fn as_ref(&self) -> &str {
        AsRef::<Arc<str>>::as_ref(self)
    }
}

impl TryFrom<&str> for JourneyId {
    type Error = EmptyJourneyId;

    fn try_from(text: &str) -> Result<Self, EmptyJourneyId> {
        Self::try_new(text.into())
    }
}

impl TryFrom<String> for JourneyId {
    type Error = EmptyJourneyId;

    fn try_from(text: String) -> Result<Self, EmptyJourneyId> {
        Self::try_new(text.into())
    }
}

/// Why a string cannot be a journey ID: it is empty.
///
/// # Examples
///
/// ```
/// use itinera::journey::{EmptyJourneyId, JourneyId};
///
/// assert_eq!(JourneyId::try_from(""), Err(EmptyJourneyId));
/// assert_eq!(EmptyJourneyId.to_string(), "a journey ID cannot be empty");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("a journey ID cannot be empty")]
pub struct EmptyJourneyId;

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::borrowed(JourneyId::try_from(""))]
    #[case::owned(JourneyId::try_from(String::new()))]
    fn a_journey_id_cannot_be_empty(#[case] made: Result<JourneyId, EmptyJourneyId>) {
        assert_eq!(made, Err(EmptyJourneyId));
    }

    #[rstest]
    #[case::borrowed(JourneyId::try_from(" "))]
    #[case::owned(JourneyId::try_from(" ".to_owned()))]
    fn a_journey_id_keeps_any_text_that_is_not_empty(
        #[case] made: Result<JourneyId, EmptyJourneyId>,
    ) {
        assert_eq!(made.map(|id| id.to_string()), Ok(" ".to_owned()));
    }

    fn text_of(text: &impl AsRef<str>) -> &str {
        text.as_ref()
    }

    #[test]
    fn a_journey_id_reads_as_its_text() {
        let id = JourneyId::try_from("order-42").unwrap();
        assert_eq!(text_of(&id), "order-42");
    }
}
