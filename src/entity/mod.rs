pub mod column;
pub mod model;
pub mod relation;

use column::ComparableColumn;
use model::Model;

pub trait Entity {
    type PrimaryKeyColumn: ComparableColumn<Entity = Self>;

    type Model: Model;

    /// The name of this entity's table in the database.
    const TABLE_NAME: &'static str;
}
