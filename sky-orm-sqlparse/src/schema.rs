use serde::{Deserialize, Serialize};
use sqlparser::ast::{ColumnDef, ColumnOption, CreateTable, DataType, ObjectNamePart};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SqlColumn {
    pub name: String,
    pub column_type: DataType,
    pub nullable: bool,
    pub unique: bool,
}

impl From<&ColumnDef> for SqlColumn {
    fn from(value: &ColumnDef) -> Self {
        Self {
            name: value.name.value.clone(),
            column_type: value.data_type.clone(),
            nullable: value
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
            unique: value
                .options
                .iter()
                .find_map(|e| {
                    if let ColumnOption::Unique {
                        is_primary: _,
                        characteristics: _,
                    } = e.option
                    {
                        Some(true)
                    } else {
                        None
                    }
                })
                .unwrap_or(false),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SqlTable {
    pub name: String,
    pub columns: Vec<SqlColumn>,
}

impl SqlTable {
    pub fn find_column(&self, name: &str) -> Option<&SqlColumn> {
        self.columns.iter().find(|e| e.name.eq(name))
    }
}

impl From<&CreateTable> for SqlTable {
    fn from(create_table: &CreateTable) -> Self {
        let columns = create_table.columns.iter().map(SqlColumn::from).collect();

        SqlTable {
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
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SqlSchema {
    pub tables: Vec<SqlTable>,
}

impl SqlSchema {
    pub fn find_table(&self, name: &str) -> Option<&SqlTable> {
        self.tables.iter().find(|e| e.name.eq(name))
    }
}
