//! The data a journey starts with, given as a type of the application's own.

use crate::value::AnyValue;

/// Data a journey starts with: each key of the data bag and its value, given to an instance with
/// [`InstanceBuilder::initial_data`](crate::instance::InstanceBuilder::initial_data).
///
/// `#[derive(InitialData)]` implements it, giving each field under the key it names. An
/// application may also implement it by hand.
///
/// # Examples
///
/// ```
/// use itinera::journey::InitialData;
/// use itinera::value::AnyValue;
///
/// struct NewOrder {
///     price: i64,
/// }
///
/// impl InitialData for NewOrder {
///     fn entries(self) -> impl IntoIterator<Item = (String, AnyValue)> {
///         [("price".to_owned(), AnyValue::new(self.price))]
///     }
/// }
/// ```
pub trait InitialData {
    /// Each key the data bag starts with, and its value.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::InitialData;
    ///
    /// fn keys(data: impl InitialData) -> Vec<String> {
    ///     data.entries().into_iter().map(|(key, _)| key).collect()
    /// }
    /// ```
    fn entries(self) -> impl IntoIterator<Item = (String, AnyValue)>;
}
