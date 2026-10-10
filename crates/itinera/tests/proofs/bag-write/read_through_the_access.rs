use itinera::error::Error;
use itinera::journey::Read;
use itinera::policy::{HookNeeds, InputAdapter, Requested};
use itinera::step::step_name;
use itinera::value::AnyValue;
use itinera::workflow::InputAdapterDescriptor;

struct Orders;

impl Orders {
    fn pricing(&self, mut got: Requested<'_, Self, InputAdapter>) -> Result<Option<AnyValue>, Error> {
        match got.data_bag()?.read::<i64>("amount") {
            Read::Present(amount) => Ok(Some(AnyValue::new(amount))),
            Read::Absent | Read::OtherType => Ok(None),
        }
    }
}

fn main() {
    let _pricing = InputAdapterDescriptor::new("pricing", step_name!("charge"), Orders::pricing)
        .needing(HookNeeds::new().data_bag());
}
