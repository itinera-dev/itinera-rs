use itinera::error::Error;
use itinera::step::{FromRescueRequests, Input, RescueNeeds, RescueOutcome, RescueResolved, rescue_step};
use itinera::value::Value;

struct Refresh<T> {
    client: T,
}

const CLIENT: &str = "client";

impl<T: Value> FromRescueRequests for Refresh<T> {
    fn needs() -> RescueNeeds {
        RescueNeeds::new().input(&Input::<T>::new(CLIENT))
    }

    fn from_requests<M>(got: &mut RescueResolved<'_, M>) -> Result<Self, Error> {
        Ok(Self {
            client: got.input(&Input::new(CLIENT))?,
        })
    }
}

#[rescue_step]
impl<T: Value> Refresh<T> {
    fn run(self) -> Result<RescueOutcome, Error> {
        let _ = self.client;
        Self::succeed()
    }
}

fn main() {}
