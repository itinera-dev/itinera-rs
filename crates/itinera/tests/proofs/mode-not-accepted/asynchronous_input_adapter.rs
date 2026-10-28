use itinera::error::Error;
use itinera::policy::Requested;
use itinera::step::step_name;
use itinera::workflow::{InputAdapter, InputAdapterDescriptor, Supplied};

struct Orders;

impl Orders {
    async fn pricing(&self, _got: Requested<'_, Self, InputAdapter>) -> Result<Supplied, Error> {
        Ok(Supplied::value(42_i64))
    }
}

fn main() {
    let _pricing = InputAdapterDescriptor::new("pricing", step_name!("charge"), Orders::pricing);
}
