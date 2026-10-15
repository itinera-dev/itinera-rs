use itinera::error::Error;
use itinera::journey::Read;
use itinera::policy::{HookNeeds, Requested};
use itinera::step::step_name;
use itinera::workflow::{InputAdapter, InputAdapterDescriptor, Supplied};

struct Orders;

impl Orders {
    fn pricing(&self, mut got: Requested<'_, Self, InputAdapter>) -> Result<Supplied, Error> {
        match got.data_bag()?.read::<i64>("price") {
            Read::Present(price) => Ok(Supplied::value(price)),
            Read::Absent | Read::OtherType => Ok(Supplied::FromDataBag),
        }
    }
}

pub(crate) fn main() {
    let _pricing = InputAdapterDescriptor::new("pricing", step_name!("charge"), Orders::pricing)
        .needing(HookNeeds::new().data_bag());
}
