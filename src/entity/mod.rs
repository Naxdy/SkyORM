use column::{Column, ComparableColumn};

pub mod column;
pub mod relation;

pub trait Entity {
    type PrimaryKeyColumn: ComparableColumn<Entity = Self>;

    /// The name of this entity's table in the database.
    const TABLE_NAME: &'static str;

    /// Iterator over all of this entity's columns.
    fn columns() -> impl IntoIterator<Item: Column<Entity = Self>>;
}
