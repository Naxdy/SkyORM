pub mod column;
pub mod model;
pub mod relation;

use column::ComparableColumn;
use model::Model;

use crate::query::select::Select;

pub trait Entity: Sized {
    type PrimaryKeyColumn: ComparableColumn<Entity = Self>;

    type Model: Model;

    /// The name of this entity's table in the database.
    const TABLE_NAME: &'static str;

    const COLUMN_NAMES: &[&'static str];

    fn find() -> Select<Self> {
        Select::<Self>::new()
    }
}
