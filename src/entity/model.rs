use sqlx::any::AnyRow;

use super::Entity;

pub trait Model {
    type Entity: Entity;

    fn from_row(row: &AnyRow) -> Self;

    fn into_active(self) -> impl ActiveModel<Model = Self>;
}

pub trait ActiveModel {
    type Model: Model;
}
