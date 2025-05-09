use sqlparser::{
    ast::{ColumnOption, ObjectNamePart, Statement},
    dialect::SQLiteDialect,
    parser::{Parser, ParserError},
};

use crate::schema::{SqlColumn, SqlTable};

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

    let columns = create_table
        .columns
        .iter()
        .map(|e| SqlColumn {
            name: e.name.value.clone(),
            column_type: e.data_type.clone(),
            nullable: e
                .options
                .iter()
                .find_map(|e| {
                    if let ColumnOption::Null = e.option {
                        Some(true)
                    } else if let ColumnOption::NotNull = e.option {
                        Some(false)
                    } else {
                        None
                    }
                })
                .unwrap_or(true),
        })
        .collect();

    Ok(SqlTable {
        name: create_table
            .name
            .0
            .iter()
            .map(|e| {
                let ObjectNamePart::Identifier(ident) = e;

                ident.value.clone()
            })
            .next()
            .unwrap(),
        columns,
    })
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
