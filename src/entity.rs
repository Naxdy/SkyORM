pub mod column;
pub mod model;
pub mod relation;

use column::ComparableColumn;
use model::Model;
use sqlx::Database;

use crate::query::{parse::ParseFromRow, select::Select};

pub trait Entity: Sized {
    type PrimaryKeyColumn: ComparableColumn<Entity = Self>;

    type Model: Model + ParseFromRow<Self::Database>;

    type Database: Database;

    /// The name of this entity's table in the database.
    const TABLE_NAME: &'static str;

    const COLUMN_NAMES: &[&'static str];

    fn find() -> Select<Self> {
        Select::new()
    }
}
