use super::Reporter;
use crate::error::Error;
use crate::journey::JourneyId;

pub trait Held {
    fn set_journey_id(&mut self, journey_id: &JourneyId) -> Result<(), Error>;
}

impl Held for Box<dyn Reporter> {
    fn set_journey_id(&mut self, journey_id: &JourneyId) -> Result<(), Error> {
        Reporter::set_journey_id(self.as_mut(), journey_id)
    }
}
