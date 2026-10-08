//! The tables of the cases, read row by row.

use std::str::FromStr;

use cucumber::gherkin;

use super::ModelError;

/// One row of a table, by the names in its header. An empty cell counts as no cell.
#[derive(Debug)]
pub(crate) struct Row {
    cells: Vec<(String, String)>,
}

impl Row {
    /// The row of these cells, each under the column in the same position of the header.
    fn under(header: &[String], cells: &[String]) -> Self {
        header.iter().cloned().zip(cells.iter().cloned()).collect()
    }

    pub(crate) fn optional(&self, column: &'static str) -> Option<&str> {
        self.cells
            .iter()
            .find(|cell| is_under(cell, column))
            .and_then(|(_, cell)| filled(cell))
    }

    pub(crate) fn columns(&self) -> impl Iterator<Item = &str> {
        self.cells.iter().map(|(name, _)| name.as_str())
    }

    pub(crate) fn required(&self, column: &'static str) -> Result<&str, ModelError> {
        self.optional(column).ok_or(ModelError::MissingCell(column))
    }

    pub(crate) fn parse<T: FromStr>(&self, column: &'static str) -> Result<T, ModelError> {
        let cell = self.required(column)?;
        cell.parse()
            .map_err(|_| ModelError::Cell(column, cell.to_owned()))
    }
}

impl FromIterator<(String, String)> for Row {
    fn from_iter<I: IntoIterator<Item = (String, String)>>(cells: I) -> Self {
        Self {
            cells: cells.into_iter().collect(),
        }
    }
}

/// The rows of a sentence's table, without its header.
pub(crate) fn rows(step: &gherkin::Step) -> Result<Vec<Row>, ModelError> {
    let table = step.table.as_ref().ok_or(ModelError::MissingTable)?;
    let mut rows = table.rows.iter();
    let header = rows.next().ok_or(ModelError::MissingTable)?;
    Ok(rows.map(|cells| Row::under(header, cells)).collect())
}

/// Whether the cell is under this column.
fn is_under((name, _): &&(String, String), column: &str) -> bool {
    name == column
}

/// The text of a cell, unless it is empty.
fn filled(cell: &str) -> Option<&str> {
    (!cell.is_empty()).then_some(cell)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(cells: &[(&str, &str)]) -> Row {
        cells.iter().map(owned).collect()
    }

    fn owned(&(name, cell): &(&str, &str)) -> (String, String) {
        (name.to_owned(), cell.to_owned())
    }

    #[test]
    fn an_empty_cell_counts_as_no_cell() {
        let row = row(&[("code", ""), ("message", "late")]);
        assert_eq!(row.optional("code"), None);
        assert_eq!(
            row.required("code").unwrap_err(),
            ModelError::MissingCell("code")
        );
        assert_eq!(row.optional("message"), Some("late"));
    }

    #[test]
    fn a_cell_is_parsed_into_the_type_asked_for() {
        let row = row(&[("attempt", "2"), ("count", "two")]);
        assert_eq!(row.parse::<u32>("attempt").unwrap(), 2);
        assert_eq!(
            row.parse::<u32>("count").unwrap_err(),
            ModelError::Cell("count", "two".to_owned())
        );
    }
}
