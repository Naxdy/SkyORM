use sqlparser::{
    ast::Statement,
    dialect::SQLiteDialect,
    parser::{Parser, ParserError},
};

use crate::schema::SqlTable;

/// Parses SQL text containing one or more `CREATE TABLE` statements and returns a list of
/// [`SqlTable`] for each parsed statement.
///
/// # Errors
///
/// If the query cannot be parsed correctly. See [`ParserError`] for more information.
pub fn parse_tables(query: &str) -> Result<Vec<SqlTable>, ParserError> {
    let ast = Parser::parse_sql(&SQLiteDialect {}, query)?;

    Ok(ast
        .iter()
        .filter_map(|e| {
            if let Statement::CreateTable(statement) = e {
                Some(statement.into())
            } else {
                None
            }
        })
        .collect())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod test {
    use super::parse_tables;

    #[test]
    fn test_create_table() {
        let query = "CREATE TABLE `test`(
          `id` INTEGER NOT NULL PRIMARY KEY,
          `name` TEXT NOT NULL,
          `something_nullable` TEXT
        )";

        let tables = parse_tables(query).expect("Failed to parse query");

        let parsed = tables.first().expect("Failed to get first table");

        assert_eq!(parsed.name, "test");
        assert!(
            parsed
                .columns
                .iter()
                .any(|e| e.name.eq("id") && !e.nullable)
        );
        assert!(
            parsed
                .columns
                .iter()
                .any(|e| e.name.eq("name") && !e.nullable)
        );
        assert!(
            parsed
                .columns
                .iter()
                .any(|e| e.name.eq("something_nullable") && e.nullable)
        );
    }
}
