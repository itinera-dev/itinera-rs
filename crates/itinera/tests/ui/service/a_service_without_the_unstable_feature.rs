use itinera::error::Error;
use itinera::step::{Outcome, Step, step};

trait StockStore: Send + Sync {
    fn reserve(&self) -> bool;
}

#[derive(Step)]
struct Ship;

#[step]
impl Ship {
    fn run(self, #[service] stock: &dyn StockStore) -> Result<Outcome, Error> {
        if stock.reserve() {
            Self::succeed()
        } else {
            Self::fail("out-of-stock")
        }
    }
}

fn main() {}
