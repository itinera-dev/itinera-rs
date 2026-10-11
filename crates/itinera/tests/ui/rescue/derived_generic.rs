use itinera::error::Error;
use itinera::step::{RescueOutcome, RescueStep, rescue_step};

#[derive(RescueStep)]
struct Refresh<T> {
    #[input]
    client: T,
}

#[rescue_step]
impl Refresh<String> {
    fn run(self) -> Result<RescueOutcome, Error> {
        Self::succeed()
    }
}

fn main() {}
