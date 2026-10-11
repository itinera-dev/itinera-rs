use itinera::error::Error;
use itinera::step::{Outcome, Step, step};
use itinera::workflow::workflow;

trait StockStore: Send + Sync {
    fn reserve(&self) -> bool;
}

struct Warehouse;

impl StockStore for Warehouse {
    fn reserve(&self) -> bool {
        true
    }
}

#[derive(Step)]
struct Ship;

#[step]
impl Ship {
    fn run(self) -> Result<Outcome, Error> {
        Self::succeed()
    }
}

#[workflow]
#[with_step(Ship, name = "ship")]
struct Orders;

#[workflow]
impl Orders {
    #[provider]
    fn stock(&self) -> Result<Box<dyn StockStore>, Error> {
        Ok(Box::new(Warehouse))
    }
}

fn main() {}
