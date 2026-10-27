use itinera::step::step_name;
use itinera::workflow::{InputAdapterDescriptor, Supplied};

struct Orders;

fn main() {
    let _pricing = InputAdapterDescriptor::new("pricing", step_name!("charge"), |_: &Orders, _| {
        Ok(None::<Supplied>)
    });
}
