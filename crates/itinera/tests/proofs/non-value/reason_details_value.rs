use itinera::step::{Outcome, Reason};

pub(crate) fn main() {
    let _declined = Outcome::failure(Reason::new("declined").with_details(1_i64));
}
