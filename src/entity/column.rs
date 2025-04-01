use std::{fmt::Display, marker::PhantomData};

use crate::{
    entity::Entity,
    query::{
        BinaryExpr, BinaryExprOperand, BracketsExpr, PushToQuery, QueryVariable, SingletonExpr,
    },
};
use sqlx::{Any, Decode, Encode, Row, Type, any::AnyRow};

pub struct ColumnName {
    table_or_alias: Option<String>,
    column_name: String,
}

impl ColumnName {
    pub(crate) fn new_with_table_or_alias(table_or_alias: String, column_name: String) -> Self {
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
    fn push_to(&mut self, builder: &mut sqlx::QueryBuilder<'_, Any>) {
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
    /// Chain another [`EntityConditionExpr`] using an `AND` statement.
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

    /// Chain another [`EntityConditionExpr`] using an `OR` statement.
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

    /// Wrap the current condition into brackets `()`.
    ///
    /// Note that calling [`Select::filter`](crate::query::select::Select::filter) will
    /// automatically place the passed condition into brackets already.
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
    fn push_to(&mut self, builder: &mut sqlx::QueryBuilder<'_, Any>) {
        self.inner.push_to(builder)
    }
}

pub trait Column {
    /// The underlying rust type of this column.
    type Type: for<'a> Encode<'a, Any> + for<'a> Decode<'a, Any> + Type<Any> + Clone;

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

    /// Try to parse a return value from a sqlx row into this column's rust type.
    fn value_from_row(row: &AnyRow) -> Result<Self::Type, sqlx::Error> {
        row.try_get(Self::full_column_name().to_string().as_str())
    }
}

pub trait NullableColumn: Column + Sized {
    /// Check whether this column is `null`.
    ///
    /// SQL: `column IS NULL`
    fn is_null() -> EntityConditionExpr<impl PushToQuery, Self::Entity> {
        SingletonExpr::new(
            Self::full_column_name(),
            crate::query::SingletonExprOperand::IsNull,
        )
        .into()
    }

    /// Check whether this column is _not_ `null` (whether it has any value stored in it).
    ///
    /// SQL: `column IS NOT NULL`
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
    /// Check whether this column equals some other value.
    ///
    /// Note that supplying [`None`] in case [`Type`](Column::Type) is an [`Option`]
    /// is _not_ equivalent to calling [`is_null`](NullableColumn::is_null), because
    /// this call will instead produce the SQL `column = NULL`, as opposed to `column IS NULL`.
    fn eq(other: Self::Type) -> EntityConditionExpr<impl PushToQuery, Self::Entity>;

    /// Check whether this column does _not_ equal some other value.
    ///
    /// Note that supplying [`None`] in case [`Type`](Column::Type) is an [`Option`]
    /// is _not_ equivalent to calling [`is_not_null`](NullableColumn::is_not_null), because
    /// this call will instead produce the SQL `column != NULL`, as opposed to `column IS NOT NULL`.
    fn not_eq(other: Self::Type) -> EntityConditionExpr<impl PushToQuery, Self::Entity>;

    /// Check whether the value of this column occurs in some collection.
    fn is_in(
        other: impl IntoIterator<Item = Self::Type>,
    ) -> EntityConditionExpr<impl PushToQuery, Self::Entity>;

    /// Check whether the value of this column does _not_ occur in some collection.
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

pub trait OrderableColumn: Column + Sized {
    /// Check whether the value of this column falls inside the range of `left` and `right`.
    fn between(
        left: Self::Type,
        right: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery, Self::Entity>;

    /// Check whether the value of this column falls outside the range of `left` and `right`.
    fn not_between(
        left: Self::Type,
        right: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery, Self::Entity>;

    /// Check whether the value of this column is greater than `other`.
    fn gt(other: Self::Type) -> EntityConditionExpr<impl PushToQuery, Self::Entity>;

    /// Check whether the value of this column is less than than `other`.
    fn lt(other: Self::Type) -> EntityConditionExpr<impl PushToQuery, Self::Entity>;

    /// Check whether the value of this column is greater than or equal to `other`.
    fn geq(other: Self::Type) -> EntityConditionExpr<impl PushToQuery, Self::Entity>;

    /// Check whether the value of this column is less than or equal to`other`.
    fn leq(other: Self::Type) -> EntityConditionExpr<impl PushToQuery, Self::Entity>;
}

impl<T> OrderableColumn for T
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

    fn gt(other: Self::Type) -> EntityConditionExpr<impl PushToQuery, Self::Entity> {
        BinaryExpr::new(
            Self::full_column_name(),
            QueryVariable(other),
            BinaryExprOperand::Gt,
        )
        .into()
    }

    fn lt(other: Self::Type) -> EntityConditionExpr<impl PushToQuery, Self::Entity> {
        BinaryExpr::new(
            Self::full_column_name(),
            QueryVariable(other),
            BinaryExprOperand::Lt,
        )
        .into()
    }

    fn geq(other: Self::Type) -> EntityConditionExpr<impl PushToQuery, Self::Entity> {
        BinaryExpr::new(
            Self::full_column_name(),
            QueryVariable(other),
            BinaryExprOperand::Geq,
        )
        .into()
    }

    fn leq(other: Self::Type) -> EntityConditionExpr<impl PushToQuery, Self::Entity> {
        BinaryExpr::new(
            Self::full_column_name(),
            QueryVariable(other),
            BinaryExprOperand::Leq,
        )
        .into()
    }
}
