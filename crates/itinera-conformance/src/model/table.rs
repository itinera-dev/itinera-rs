//! The tables of the cases, read row by row.

use std::str::FromStr;

use cucumber::gherkin;

use super::ModelError;

/// One row of a table, by the names in its header. An empty cell counts as no cell.
#[derive(Debug)]
pub(crate) struct Row(Vec<(String, String)>);

impl Row {
    pub(crate) fn optional(&self, column: &'static str) -> Option<&str> {
        self.0
            .iter()
            .find(|(name, _)| name == column)
            .map(|(_, cell)| cell.as_str())
            .filter(|cell| !cell.is_empty())
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

/// The rows of a sentence's table, without its header.
pub(crate) fn rows(step: &gherkin::Step) -> Result<Vec<Row>, ModelError> {
    let table = step.table.as_ref().ok_or(ModelError::MissingTable)?;
    let mut rows = table.rows.iter();
    let header = rows.next().ok_or(ModelError::MissingTable)?;
    Ok(rows
        .map(|cells| Row(header.iter().cloned().zip(cells.iter().cloned()).collect()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(cells: &[(&str, &str)]) -> Row {
        Row(cells
            .iter()
            .map(|(name, cell)| ((*name).to_owned(), (*cell).to_owned()))
            .collect())
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
