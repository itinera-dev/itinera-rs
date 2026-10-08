#[cfg(feature = "async")]
use crate::report::BoxedReporter;

pub trait Sealed {
    /// A reporter as the asynchronous executor holds it, whatever the workflow's mode.
    #[cfg(feature = "async")]
    fn boxed(reporter: <Self as super::Mode>::Reporter) -> BoxedReporter
    where
        Self: super::Mode;
}

impl Sealed for super::Synchronous {
    #[cfg(feature = "async")]
    fn boxed(reporter: Box<dyn crate::report::Reporter>) -> BoxedReporter {
        BoxedReporter::from(reporter)
    }
}
