//! The names of a workflow and of its input adapters, fixed when the program is compiled.

/// A workflow's name, fixed when the program is compiled.
///
/// # Examples
///
/// ```
/// use itinera::workflow::WorkflowName;
///
/// let orders = WorkflowName::from("orders");
/// let text: &str = orders.as_ref();
/// assert_eq!(text, "orders");
/// assert_eq!(orders.to_string(), "orders");
/// ```
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
    derive_more::AsRef,
)]
#[as_ref(forward)]
pub struct WorkflowName(&'static str);

/// An input adapter's name, fixed when the program is compiled, unique among its workflow's
/// adapters.
///
/// # Examples
///
/// ```
/// use itinera::workflow::AdapterName;
///
/// let pricing = AdapterName::from("pricing");
/// let text: &str = pricing.as_ref();
/// assert_eq!(text, "pricing");
/// assert_eq!(pricing.to_string(), "pricing");
/// ```
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
    derive_more::AsRef,
)]
#[as_ref(forward)]
pub struct AdapterName(&'static str);
