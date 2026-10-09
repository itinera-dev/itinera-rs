use itinera::step::{Outcome, Reason};

fn main() {
    let _declined = Outcome::failure(Reason::new("declined").with_details(|| 1));
}
