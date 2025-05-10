use sqlparser::{
    ast::Statement,
    dialect::SQLiteDialect,
    parser::{Parser, ParserError},
};

use crate::schema::SqlTable;

pub fn parse_create_table(query: &str) -> Result<SqlTable, ParserError> {
    let ast = Parser::parse_sql(&SQLiteDialect {}, query)?;

    let create_table = ast
        .iter()
        .find_map(|e| {
            if let Statement::CreateTable(statement) = e {
                Some(statement)
            } else {
                None
            }
        })
        .unwrap();

    Ok(create_table.into())
}

#[cfg(test)]
mod test {
    use super::parse_create_table;

    #[test]
    fn test_create_table() {
        let query = "CREATE TABLE `test`(
          `id` INTEGER NOT NULL PRIMARY KEY,
          `name` TEXT NOT NULL,
          `something_nullable` TEXT
        )";

        let parsed = parse_create_table(query).expect("Failed to parse query");

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
