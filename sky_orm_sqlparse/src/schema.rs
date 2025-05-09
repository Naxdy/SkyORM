use serde::{Deserialize, Serialize};
use sqlparser::ast::DataType;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "name")]
pub struct SqlColumn {
    pub name: String,
    pub column_type: DataType,
    pub nullable: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "name")]
pub struct SqlTable {
    pub name: String,
    pub columns: Vec<SqlColumn>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SqlSchema {
    pub tables: Vec<SqlTable>,
}
