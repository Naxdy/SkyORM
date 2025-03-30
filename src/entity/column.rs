use std::{fmt::Display, marker::PhantomData};

use crate::{
    entity::Entity,
    query::{
        BinaryExpr, BinaryExprOperand, BracketsExpr, PushToQuery, QueryVariable, SingletonExpr,
    },
};
use sqlx::{Any, Decode, Encode, Row, Type, any::AnyRow};

pub enum ColumnExprOperand {
    Equals,
    DoesNotEqual,
    Like,
    ILike,
    IsNull,
    IsNotNull,
}

impl Display for ColumnExprOperand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Equals => "=",
                Self::DoesNotEqual => "!=",
                Self::Like => "LIKE",
                Self::ILike => "ILIKE",
                Self::IsNull => "IS NULL",
                Self::IsNotNull => "IS NOT NULL",
            }
        )
    }
}

pub struct ColumnName {
    table_or_alias: Option<String>,
    column_name: String,
}

impl ColumnName {
    pub fn new(column_name: String) -> Self {
        Self {
            table_or_alias: None,
            column_name,
        }
    }

    pub fn new_with_table_or_alias(table_or_alias: String, column_name: String) -> Self {
        Self {
            table_or_alias: Some(table_or_alias),
            column_name,
        }
    }
}

impl Display for ColumnName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(table_or_alias) = &self.table_or_alias {
            write!(f, "\"{}\".", table_or_alias)?;
        }
        write!(f, "\"{}\"", self.column_name)
    }
}

impl PushToQuery for ColumnName {
    fn push_to(self, builder: &mut sqlx::QueryBuilder<'_, Any>) {
        builder.push(self.to_string());
    }
}

pub struct EntityConditionExpr<Q, E>
where
    Q: PushToQuery,
    E: Entity,
{
    marker: PhantomData<E>,
    inner: Q,
}

impl<Q, E> EntityConditionExpr<Q, E>
where
    Q: PushToQuery,
    E: Entity,
{
    pub fn and<OQ>(
        self,
        other: EntityConditionExpr<OQ, E>,
    ) -> EntityConditionExpr<BinaryExpr<Q, EntityConditionExpr<OQ, E>>, E>
    where
        OQ: PushToQuery,
    {
        EntityConditionExpr {
            marker: PhantomData,
            inner: BinaryExpr::new(self.inner, other, BinaryExprOperand::And),
        }
    }

    pub fn or<OQ>(
        self,
        other: EntityConditionExpr<OQ, E>,
    ) -> EntityConditionExpr<BinaryExpr<Q, EntityConditionExpr<OQ, E>>, E>
    where
        OQ: PushToQuery,
    {
        EntityConditionExpr {
            marker: PhantomData,
            inner: BinaryExpr::new(self.inner, other, BinaryExprOperand::Or),
        }
    }

    /// Wrap the query into brackets `()`.
    pub fn brackets(self) -> EntityConditionExpr<BracketsExpr<Q>, E> {
        EntityConditionExpr {
            marker: PhantomData,
            inner: BracketsExpr::new(self.inner),
        }
    }
}

impl<Q, E> From<Q> for EntityConditionExpr<Q, E>
where
    Q: PushToQuery,
    E: Entity,
{
    fn from(value: Q) -> Self {
        Self {
            inner: value,
            marker: PhantomData,
        }
    }
}

impl<Q, E> PushToQuery for EntityConditionExpr<Q, E>
where
    Q: PushToQuery,
    E: Entity,
{
    fn push_to(self, builder: &mut sqlx::QueryBuilder<'_, Any>) {
        self.inner.push_to(builder)
    }
}

pub trait Column {
    /// The underlying rust type of this column.
    type Type: for<'a> Encode<'a, Any> + for<'a> Decode<'a, Any> + Type<Any>;

    /// The entity that this column belongs to.
    type Entity: Entity;

    /// The name this column has in the database;
    const NAME: &'static str;

    /// The fully qualified name of this column, usually something like
    /// `"entity_table_name"."column_name"`.
    fn full_column_name() -> ColumnName {
        ColumnName::new_with_table_or_alias(
            Self::Entity::TABLE_NAME.to_string(),
            Self::NAME.to_string(),
        )
    }

    /// Parse a return value from a sqlx row into this column's rust type.
    fn value_from_row(row: &AnyRow) -> Self::Type {
        row.get(Self::full_column_name().to_string().as_str())
    }
}

pub trait NullableColumn: Column + Sized {
    fn is_null() -> EntityConditionExpr<impl PushToQuery, Self::Entity> {
        SingletonExpr::new(
            Self::full_column_name(),
            crate::query::SingletonExprOperand::IsNull,
        )
        .into()
    }

    fn is_not_null() -> EntityConditionExpr<impl PushToQuery, Self::Entity> {
        SingletonExpr::new(
            Self::full_column_name(),
            crate::query::SingletonExprOperand::IsNotNull,
        )
        .into()
    }
}

impl<T, Type> NullableColumn for T where T: Column<Type = Option<Type>> {}

pub trait ComparableColumn: Column + Sized {
    fn eq(other: Self::Type) -> EntityConditionExpr<impl PushToQuery, Self::Entity>;

    fn not_eq(other: Self::Type) -> EntityConditionExpr<impl PushToQuery, Self::Entity>;

    fn is_in(
        other: impl IntoIterator<Item = Self::Type>,
    ) -> EntityConditionExpr<impl PushToQuery, Self::Entity>;

    fn is_not_in(
        other: impl IntoIterator<Item = Self::Type>,
    ) -> EntityConditionExpr<impl PushToQuery, Self::Entity>;
}

impl<T> ComparableColumn for T
where
    T: Column,
    T::Type: PartialEq + 'static,
{
    fn eq(other: Self::Type) -> EntityConditionExpr<impl PushToQuery, Self::Entity> {
        BinaryExpr::new(
            Self::full_column_name(),
            QueryVariable(other),
            crate::query::BinaryExprOperand::Equals,
        )
        .into()
    }

    fn not_eq(other: Self::Type) -> EntityConditionExpr<impl PushToQuery, Self::Entity> {
        BinaryExpr::new(
            Self::full_column_name(),
            QueryVariable(other),
            crate::query::BinaryExprOperand::DoesNotEqual,
        )
        .into()
    }

    fn is_in(
        other: impl IntoIterator<Item = Self::Type>,
    ) -> EntityConditionExpr<impl PushToQuery, Self::Entity> {
        BinaryExpr::new(
            Self::full_column_name(),
            other.into_iter().map(QueryVariable).collect::<Vec<_>>(),
            crate::query::BinaryExprOperand::In,
        )
        .into()
    }

    fn is_not_in(
        other: impl IntoIterator<Item = Self::Type>,
    ) -> EntityConditionExpr<impl PushToQuery, Self::Entity> {
        BinaryExpr::new(
            Self::full_column_name(),
            other.into_iter().map(QueryVariable).collect::<Vec<_>>(),
            crate::query::BinaryExprOperand::NotIn,
        )
        .into()
    }
}

pub trait StringComparableColumn: Column + Sized {
    fn like(other: impl Into<String>) -> EntityConditionExpr<impl PushToQuery, Self::Entity> {
        BinaryExpr::new(
            Self::full_column_name(),
            other.into(),
            crate::query::BinaryExprOperand::Like,
        )
        .into()
    }

    fn ilike(other: impl Into<String>) -> EntityConditionExpr<impl PushToQuery, Self::Entity> {
        BinaryExpr::new(
            Self::full_column_name(),
            other.into(),
            crate::query::BinaryExprOperand::ILike,
        )
        .into()
    }
}

impl<T> StringComparableColumn for T
where
    T: Column,
    T::Type: Into<String>,
{
}

pub trait RangeColumn: Column + Sized {
    fn between(
        left: Self::Type,
        right: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery, Self::Entity>;

    fn not_between(
        left: Self::Type,
        right: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery, Self::Entity>;
}

impl<T> RangeColumn for T
where
    T: Column,
    T::Type: PartialOrd + 'static,
{
    fn between(
        left: Self::Type,
        right: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery, Self::Entity> {
        BinaryExpr::new(
            Self::full_column_name(),
            BinaryExpr::new(
                QueryVariable(left),
                QueryVariable(right),
                BinaryExprOperand::And,
            ),
            BinaryExprOperand::Between,
        )
        .into()
    }

    fn not_between(
        left: Self::Type,
        right: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery, Self::Entity> {
        BinaryExpr::new(
            Self::full_column_name(),
            BinaryExpr::new(
                QueryVariable(left),
                QueryVariable(right),
                BinaryExprOperand::And,
            ),
            BinaryExprOperand::NotBetween,
        )
        .into()
    }
}
