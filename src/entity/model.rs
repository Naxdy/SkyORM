use sqlx::{Any, Decode, Encode, Type, any::AnyRow};

use crate::query::parse::ParseFromRow;

use super::Entity;

#[derive(Clone)]
pub enum ActiveModelValue<T>
where
    T: for<'a> Encode<'a, Any> + for<'a> Decode<'a, Any> + Type<Any> + Clone,
{
    Set(T),
    Unchanged(T),
    NotSet,
}

impl<T> ActiveModelValue<T>
where
    T: for<'a> Encode<'a, Any> + for<'a> Decode<'a, Any> + Type<Any> + Clone,
{
    pub fn get(&self) -> Option<&T> {
        Option::from(self)
    }

    pub fn set(&mut self, value: T) {
        *self = ActiveModelValue::Set(value);
    }

    pub fn clear(&mut self) {
        *self = ActiveModelValue::NotSet;
    }

    pub fn mark_unchanged(&mut self) {
        if let ActiveModelValue::Set(e) = self {
            *self = ActiveModelValue::Unchanged(e.clone());
        }
    }
}

impl<'m, T> From<&'m ActiveModelValue<T>> for Option<&'m T>
where
    T: for<'a> Encode<'a, Any> + for<'a> Decode<'a, Any> + Type<Any> + Clone,
{
    fn from(value: &'m ActiveModelValue<T>) -> Self {
        match value {
            ActiveModelValue::Set(e) => Some(e),
            ActiveModelValue::Unchanged(e) => Some(e),
            ActiveModelValue::NotSet => None,
        }
    }
}

pub trait Model: ParseFromRow + Sized {
    type Entity: Entity;
    type ActiveModel: ActiveModel;

    fn into_active(self) -> Self::ActiveModel;
}

pub trait ActiveModel {
    type Model: Model;
}
