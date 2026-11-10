//! Fixtures that the tests of several modules share.

use super::Row;

impl Row {
    /// The row of these cells, each under its column.
    pub(crate) fn of(cells: &[(&str, &str)]) -> Self {
        cells.iter().map(owned).collect()
    }
}

fn owned(&(name, cell): &(&str, &str)) -> (String, String) {
    (name.to_owned(), cell.to_owned())
}
