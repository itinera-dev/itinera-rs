use itinera::step::step_name;
use itinera::value::AnyValue;
use itinera::workflow::InputAdapter;

struct Orders;

fn main() {
    let _pricing = InputAdapter::new("pricing", step_name!("charge"), |_: &Orders, _, _| {
        Ok(Some(AnyValue::new(1_i64)))
    });
}
