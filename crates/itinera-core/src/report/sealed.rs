use super::Reporter;

pub trait Held {}

impl Held for Box<dyn Reporter> {}

#[cfg(feature = "async")]
impl Held for super::BoxedReporter {}
