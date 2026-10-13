use super::Reporter;
use crate::error::Error;
use crate::value::AnyValue;

pub trait Held {
    fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<(), Error>;
}

impl Held for Box<dyn Reporter> {
    fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<(), Error> {
        Reporter::set_global_metadata(self.as_mut(), key, value)
    }
}
