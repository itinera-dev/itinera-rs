use itinera::error::Error;
use itinera::journey::DataBag;
use itinera::policy::{HookNeeds, Requested};
use itinera::step::step_name;
use itinera::workflow::{InputAdapter, InputAdapterDescriptor, Supplied};

struct Orders;

impl Orders {
    fn pricing(&self, mut got: Requested<'_, Self, InputAdapter>) -> Result<Supplied, Error> {
        *got.data_bag()? = DataBag::default();
        Ok(Supplied::FromDataBag)
    }
}

fn main() {
    let _pricing = InputAdapterDescriptor::new("pricing", step_name!("charge"), Orders::pricing)
        .needing(HookNeeds::new().data_bag());
}
