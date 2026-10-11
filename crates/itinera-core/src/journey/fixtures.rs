//! Fixtures the tests of several modules share: a data bag holding two values.

use super::DataBag;
use crate::value::AnyValue;

pub(super) fn bag() -> DataBag {
    let mut data = DataBag::new();
    data.insert("customer".to_string(), AnyValue::new("ana".to_string()));
    data.insert("amount".to_string(), AnyValue::new(42_i64));
    data
}
