use std::sync::Mutex;

use itinera::error::Error;
use itinera::journey::{DataBagAccess, Read};
use itinera::policy::{HookNeeds, InputAdapter, Requested};
use itinera::step::step_name;
use itinera::value::AnyValue;
use itinera::workflow::InputAdapterDescriptor;

struct Orders {
    kept: Mutex<Option<DataBagAccess<'static>>>,
}

impl Orders {
    fn pricing(&self, mut got: Requested<'_, Self, InputAdapter>) -> Result<Option<AnyValue>, Error> {
        let access = got.data_bag()?;
        *self.kept.lock().unwrap() = Some(access);
        match access.read::<i64>("price") {
            Read::Present(price) => Ok(Some(AnyValue::new(price))),
            Read::Absent | Read::OtherType => Ok(None),
        }
    }
}

fn main() {
    let _pricing = InputAdapterDescriptor::new("pricing", step_name!("charge"), Orders::pricing)
        .needing(HookNeeds::new().data_bag());
}
