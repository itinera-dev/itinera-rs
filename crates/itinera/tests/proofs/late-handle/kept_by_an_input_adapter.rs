use std::sync::Mutex;

use itinera::error::Error;
use itinera::journey::{DataBag, Read};
use itinera::policy::{HookNeeds, InputAdapter, Requested};
use itinera::step::step_name;
use itinera::workflow::{InputAdapterDescriptor, Supplied};

struct Orders {
    kept: Mutex<Option<&'static DataBag>>,
}

impl Orders {
    fn pricing(&self, mut got: Requested<'_, Self, InputAdapter>) -> Result<Supplied, Error> {
        let access = got.data_bag()?;
        *self.kept.lock().unwrap() = Some(access);
        match access.read::<i64>("price") {
            Read::Present(price) => Ok(Supplied::value(price)),
            Read::Absent | Read::OtherType => Ok(Supplied::FromDataBag),
        }
    }
}

fn main() {
    let _pricing = InputAdapterDescriptor::new("pricing", step_name!("charge"), Orders::pricing)
        .needing(HookNeeds::new().data_bag());
}
