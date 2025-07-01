use std::{fmt::Display, marker::PhantomData};

use crate::{
    entity::Entity,
    query::{
        BinaryExpr, BinaryExprOperand, BracketsExpr, PushToQuery, QueryVariable, SingletonExpr,
    },
};
use sqlx::{ColumnIndex, Database, Decode, Encode, Row, Type};

/// A struct that represents the name of a column on a particular table.
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

    /// The name of the column within the database.
    pub fn column_name(&self) -> &String {
        &self.column_name
    }

    /// The name of the table within the database that this column is part of.
    pub fn table_or_alias(&self) -> Option<&String> {
        self.table_or_alias.as_ref()
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

impl<DB> PushToQuery<DB> for ColumnName
where
    DB: Database,
{
    fn push_to(&self, builder: &mut sqlx::QueryBuilder<'_, DB>) {
        builder.push(self.to_string());
    }
}

/// A struct that represents a conditional expression (such as `=`, `>`, `IS NULL`) on a given
/// entity `E`.
pub struct EntityConditionExpr<Q, E>
where
    Q: PushToQuery<E::Database>,
    E: Entity,
{
    marker: PhantomData<E>,
    inner: Q,
}

impl<Q, E> EntityConditionExpr<Q, E>
where
    Q: PushToQuery<E::Database>,
    E: Entity,
{
    /// Chain another [`EntityConditionExpr`] using an `AND` statement.
    pub fn and<OQ>(
        self,
        other: EntityConditionExpr<OQ, E>,
    ) -> EntityConditionExpr<impl PushToQuery<E::Database>, E>
    where
        OQ: PushToQuery<E::Database>,
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
    ) -> EntityConditionExpr<impl PushToQuery<E::Database>, E>
    where
        OQ: PushToQuery<E::Database>,
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
    pub fn brackets(self) -> EntityConditionExpr<impl PushToQuery<E::Database>, E> {
        EntityConditionExpr {
            marker: PhantomData,
            inner: BracketsExpr::new(self.inner),
        }
    }
}

impl<Q, E> From<Q> for EntityConditionExpr<Q, E>
where
    Q: PushToQuery<E::Database>,
    E: Entity,
{
    fn from(value: Q) -> Self {
        Self {
            inner: value,
            marker: PhantomData,
        }
    }
}

impl<Q, E> PushToQuery<E::Database> for EntityConditionExpr<Q, E>
where
    Q: PushToQuery<E::Database>,
    E: Entity,
{
    fn push_to(&self, builder: &mut sqlx::QueryBuilder<'_, E::Database>) {
        self.inner.push_to(builder)
    }
}

pub trait Column {
    /// The underlying rust type of this column.
    type Type: for<'a> Encode<'a, <Self::Entity as Entity>::Database>
        + for<'a> Decode<'a, <Self::Entity as Entity>::Database>
        + Type<<Self::Entity as Entity>::Database>
        + Clone;

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
    fn value_from_row<R>(row: &R) -> Result<Self::Type, sqlx::Error>
    where
        R: Row<Database = <Self::Entity as Entity>::Database>,
        for<'a> &'a str: ColumnIndex<R>,
    {
        row.try_get(Self::full_column_name().to_string().as_str())
    }
}

pub trait NullableColumn: Column + Sized {
    /// Check whether this column is `null`.
    ///
    /// SQL: `column IS NULL`
    fn is_null()
    -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity> {
        SingletonExpr::new(
            Self::full_column_name(),
            crate::query::SingletonExprOperand::IsNull,
        )
        .into()
    }

    /// Check whether this column is _not_ `null` (whether it has any value stored in it).
    ///
    /// SQL: `column IS NOT NULL`
    fn is_not_null()
    -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity> {
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
    fn eq(
        other: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>;

    /// Check whether this column does _not_ equal some other value.
    ///
    /// Note that supplying [`None`] in case [`Type`](Column::Type) is an [`Option`]
    /// is _not_ equivalent to calling [`is_not_null`](NullableColumn::is_not_null), because
    /// this call will instead produce the SQL `column != NULL`, as opposed to `column IS NOT NULL`.
    fn not_eq(
        other: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>;

    /// Check whether the value of this column occurs in some collection.
    fn is_in(
        other: impl IntoIterator<Item = Self::Type>,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>;

    /// Check whether the value of this column does _not_ occur in some collection.
    fn is_not_in(
        other: impl IntoIterator<Item = Self::Type>,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>;
}

impl<T> ComparableColumn for T
where
    T: Column,
    T::Type: PartialEq + 'static,
{
    fn eq(
        other: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>
    {
        BinaryExpr::new(
            Self::full_column_name(),
            QueryVariable::new(other),
            crate::query::BinaryExprOperand::Equals,
        )
        .into()
    }

    fn not_eq(
        other: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>
    {
        BinaryExpr::new(
            Self::full_column_name(),
            QueryVariable::new(other),
            crate::query::BinaryExprOperand::DoesNotEqual,
        )
        .into()
    }

    fn is_in(
        other: impl IntoIterator<Item = Self::Type>,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>
    {
        BinaryExpr::new(
            Self::full_column_name(),
            other
                .into_iter()
                .map(QueryVariable::new)
                .collect::<Vec<_>>(),
            crate::query::BinaryExprOperand::In,
        )
        .into()
    }

    fn is_not_in(
        other: impl IntoIterator<Item = Self::Type>,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>
    {
        BinaryExpr::new(
            Self::full_column_name(),
            other
                .into_iter()
                .map(QueryVariable::new)
                .collect::<Vec<_>>(),
            crate::query::BinaryExprOperand::NotIn,
        )
        .into()
    }
}

pub trait StringComparableColumn: Column + Sized {
    fn like(
        other: impl Into<String>,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>
    {
        BinaryExpr::new(
            Self::full_column_name(),
            other.into(),
            crate::query::BinaryExprOperand::Like,
        )
        .into()
    }

    fn ilike(
        other: impl Into<String>,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>
    {
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
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>;

    /// Check whether the value of this column falls outside the range of `left` and `right`.
    fn not_between(
        left: Self::Type,
        right: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>;

    /// Check whether the value of this column is greater than `other`.
    fn gt(
        other: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>;

    /// Check whether the value of this column is less than than `other`.
    fn lt(
        other: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>;

    /// Check whether the value of this column is greater than or equal to `other`.
    fn geq(
        other: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>;

    /// Check whether the value of this column is less than or equal to`other`.
    fn leq(
        other: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>;
}

impl<T> OrderableColumn for T
where
    T: Column,
    T::Type: PartialOrd + 'static,
{
    fn between(
        left: Self::Type,
        right: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>
    {
        BinaryExpr::new(
            Self::full_column_name(),
            BinaryExpr::new(
                QueryVariable::new(left),
                QueryVariable::new(right),
                BinaryExprOperand::And,
            ),
            BinaryExprOperand::Between,
        )
        .into()
    }

    fn not_between(
        left: Self::Type,
        right: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>
    {
        BinaryExpr::new(
            Self::full_column_name(),
            BinaryExpr::new(
                QueryVariable::new(left),
                QueryVariable::new(right),
                BinaryExprOperand::And,
            ),
            BinaryExprOperand::NotBetween,
        )
        .into()
    }

    fn gt(
        other: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>
    {
        BinaryExpr::new(
            Self::full_column_name(),
            QueryVariable::new(other),
            BinaryExprOperand::Gt,
        )
        .into()
    }

    fn lt(
        other: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>
    {
        BinaryExpr::new(
            Self::full_column_name(),
            QueryVariable::new(other),
            BinaryExprOperand::Lt,
        )
        .into()
    }

    fn geq(
        other: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>
    {
        BinaryExpr::new(
            Self::full_column_name(),
            QueryVariable::new(other),
            BinaryExprOperand::Geq,
        )
        .into()
    }

    fn leq(
        other: Self::Type,
    ) -> EntityConditionExpr<impl PushToQuery<<Self::Entity as Entity>::Database>, Self::Entity>
    {
        BinaryExpr::new(
            Self::full_column_name(),
            QueryVariable::new(other),
            BinaryExprOperand::Leq,
        )
        .into()
    }
}
