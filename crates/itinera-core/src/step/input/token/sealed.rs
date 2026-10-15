use super::InputToken;
use crate::value::AnyValue;

/// What only itinera's tokens can say: how they are declared and read.
pub trait Sealed {
    /// Whether the value may be absent.
    const OPTIONAL: bool;

    /// The token's key.
    fn key(&self) -> &'static str;

    /// What a slot holding the resolved value gives, or `None` when it was not declared with
    /// this token's type or was already taken.
    fn given(slot: Option<Option<AnyValue>>) -> Option<<Self as InputToken>::Given>
    where
        Self: InputToken;
}
