use itinera::step::step_name;
use itinera::value::AnyValue;
use itinera::workflow::InputAdapterDescriptor;

struct Orders;

fn main() {
    let _pricing = InputAdapterDescriptor::new("pricing", step_name!("charge"), |_: &Orders, _| {
        Ok(Some(AnyValue::new(|| 1)))
    });
}
