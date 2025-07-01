use std::marker::PhantomData;

use sqlx::{Database, Decode, Encode, Type};

use crate::query::parse::ParseFromRow;

use super::Entity;

#[derive(Clone)]
pub enum ActiveModelValue<T, DB>
where
    T: for<'a> Encode<'a, DB> + for<'a> Decode<'a, DB> + Type<DB> + Clone,
    DB: Database,
{
    Set(T),
    Unchanged(T),
    NotSet(PhantomData<DB>),
}

impl<T, DB> ActiveModelValue<T, DB>
where
    T: for<'a> Encode<'a, DB> + for<'a> Decode<'a, DB> + Type<DB> + Clone,
    DB: Database,
{
    pub fn get(&self) -> Option<&T> {
        Option::from(self)
    }

    pub fn set(&mut self, value: T) {
        *self = ActiveModelValue::Set(value);
    }

    pub fn clear(&mut self) {
        *self = ActiveModelValue::NotSet(PhantomData);
    }

    pub fn mark_unchanged(&mut self) {
        if let ActiveModelValue::Set(e) = self {
            *self = ActiveModelValue::Unchanged(e.clone());
        }
    }
}

impl<'m, T, DB> From<&'m ActiveModelValue<T, DB>> for Option<&'m T>
where
    T: for<'a> Encode<'a, DB> + for<'a> Decode<'a, DB> + Type<DB> + Clone,
    DB: Database,
{
    fn from(value: &'m ActiveModelValue<T, DB>) -> Self {
        match value {
            ActiveModelValue::Set(e) => Some(e),
            ActiveModelValue::Unchanged(e) => Some(e),
            ActiveModelValue::NotSet(_) => None,
        }
    }
}

pub trait Model: ParseFromRow<<Self::Entity as Entity>::Database> + Sized {
    type Entity: Entity;
    type ActiveModel: ActiveModel;

    fn into_active(self) -> Self::ActiveModel;
}

pub trait ActiveModel {
    type Model: Model;
}
