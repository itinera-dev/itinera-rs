use itinera::step::step_name;
use itinera::workflow::{InputAdapterDescriptor, Supplied};

struct Orders;

pub(crate) fn main() {
    let _pricing = InputAdapterDescriptor::new("pricing", step_name!("charge"), |_: &Orders, _| {
        Ok(Supplied::value(1_i64))
    });
}
