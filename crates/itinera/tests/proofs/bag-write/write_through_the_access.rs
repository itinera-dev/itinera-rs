use itinera::error::Error;
use itinera::policy::{HookNeeds, InputAdapter, Requested};
use itinera::step::step_name;
use itinera::value::AnyValue;
use itinera::workflow::{InputAdapterDescriptor, Supplied};

struct Orders;

impl Orders {
    fn pricing(&self, mut got: Requested<'_, Self, InputAdapter>) -> Result<Supplied, Error> {
        got.data_bag()?.insert("amount", AnyValue::new(0_i64));
        Ok(Supplied::FromDataBag)
    }
}

fn main() {
    let _pricing = InputAdapterDescriptor::new("pricing", step_name!("charge"), Orders::pricing)
        .needing(HookNeeds::new().data_bag());
}
