use std::{marker::PhantomData, sync::Arc};

use futures::StreamExt;
use itertools::Itertools;
use sqlx::{Connection, Database, Executor, IntoArguments, QueryBuilder};

use crate::entity::{
    Entity,
    column::{Column, EntityConditionExpr},
    relation::{InverseRelated, Related},
};

use super::{BinaryExpr, BinaryExprOperand, BracketsExpr, PushToQuery, parse::ParseFromRow};

pub struct Select<T>
where
    T: Entity + 'static,
{
    marker: PhantomData<T>,
    conditions: Vec<Arc<dyn PushToQuery<T::Database>>>,
    additional_tables: Vec<String>,
}

impl<T> Select<T>
where
    T: Entity + 'static,
{
    pub(crate) fn new() -> Self {
        Self {
            marker: PhantomData,
            conditions: vec![],
            additional_tables: vec![],
        }
    }

    /// Append a new `WHERE` condition using an `AND` statement as glue. The passed condition is
    /// wrapped in `()` brackets.
    #[must_use]
    pub fn filter<Q>(mut self, condition: EntityConditionExpr<Q, T>) -> Self
    where
        Q: PushToQuery<T::Database> + 'static,
    {
        self.conditions.push(Arc::new(condition));
        self
    }

    /// Append a new `WHERE` condition using an `AND` statement as glue, allowing to filter the
    /// columns of a related entity (the foreign key is on `R`). The passed condition is wrapped
    /// in `()` brackets.
    #[must_use]
    pub fn where_relation<C, Q, R>(mut self, condition: EntityConditionExpr<Q, R>) -> Self
    where
        Q: PushToQuery<T::Database> + 'static,
        R: Related<T, C, Database = T::Database> + 'static,
        T: InverseRelated<R, C>,
        C: Column<Entity = R, Type = <T::PrimaryKeyColumn as Column>::Type>,
        <T::PrimaryKeyColumn as Column>::Type: PartialEq,
    {
        self.conditions.push(Arc::new(condition));
        self.conditions.push(Arc::new(BinaryExpr::new(
            C::full_column_name(),
            <T::PrimaryKeyColumn as Column>::full_column_name(),
            BinaryExprOperand::Equals,
        )));
        self.additional_tables.push(R::TABLE_NAME.to_string());
        self
    }

    /// Append a new `WHERE` condition using an `AND` statement as glue, allowing to filter the
    /// columns of an inversely related entity (the foreign key is on `T`). The passed condition is
    /// wrapped in `()` brackets.
    #[must_use]
    pub fn where_inverse_relation<C, Q, R>(mut self, condition: EntityConditionExpr<Q, R>) -> Self
    where
        Q: PushToQuery<T::Database> + 'static,
        R: InverseRelated<T, C, Database = T::Database> + 'static,
        T: Related<R, C>,
        C: Column<Entity = T, Type = <R::PrimaryKeyColumn as Column>::Type>,
        <R::PrimaryKeyColumn as Column>::Type: PartialEq,
    {
        self.conditions.push(Arc::new(condition));
        self.conditions.push(Arc::new(BinaryExpr::new(
            C::full_column_name(),
            <R::PrimaryKeyColumn as Column>::full_column_name(),
            BinaryExprOperand::Equals,
        )));
        self.additional_tables.push(R::TABLE_NAME.to_string());
        self
    }

    /// Return the raw SQL query of this statement. Note that the returned query is
    /// backend-agnostic, e.g. query parameters will be substituted with `?` instead of `$1` (in
    /// the case of postgres).
    ///
    /// This is mainly useful for debugging purposes, and not intended to produce queries to be run
    /// on an actual database.
    #[must_use]
    pub fn query(&self) -> String {
        let mut builder = QueryBuilder::new("");
        self.push_to(&mut builder);
        builder.into_sql()
    }

    /// Execute the query, returning a single result.
    ///
    /// # Errors
    ///
    /// If no entry could be found, or if there's been a problem communicating with the database.
    /// See [`sqlx::Error`] for more information.
    pub async fn one<'c, C>(self, connection: &'c mut C) -> Result<T::Model, sqlx::Error>
    where
        C: Connection<Database = T::Database>,
        &'c mut C: Executor<'c, Database = T::Database>,
        for<'q> <T::Database as Database>::Arguments<'q>: IntoArguments<'q, T::Database> + 'c,
    {
        let mut builder = QueryBuilder::new("");
        self.push_to(&mut builder);

        drop(self);

        let result = connection.fetch_one(builder.build()).await?;
        <T::Model as ParseFromRow<T::Database>>::parse_from_row(&result)
    }

    /// Execute the query, returning all results.
    ///
    /// # Errors
    ///
    /// If there's been a problem communicating with the database. See [`sqlx::Error`] for more
    /// information.
    pub async fn all<'c, C>(self, connection: &'c mut C) -> Result<Vec<T::Model>, sqlx::Error>
    where
        C: Connection<Database = T::Database>,
        &'c mut C: Executor<'c, Database = T::Database>,
        for<'q> <T::Database as Database>::Arguments<'q>: IntoArguments<'q, T::Database> + 'c,
    {
        let mut builder = QueryBuilder::new("");
        self.push_to(&mut builder);

        drop(self);

        let result = connection
            .fetch(builder.build())
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;

        result
            .iter()
            .map(<T::Model as ParseFromRow<T::Database>>::parse_from_row)
            .collect::<Result<Vec<_>, _>>()
    }
}

impl<T> PushToQuery<T::Database> for Select<T>
where
    T: Entity + 'static,
{
    // Unwraps are checked beforehand
    #[allow(clippy::unwrap_used)]
    fn push_to(&self, builder: &mut sqlx::QueryBuilder<'_, T::Database>) {
        builder.push("SELECT ");

        T::COLUMN_NAMES.iter().enumerate().for_each(|(i, e)| {
            if i > 0 {
                builder.push(", ");
            }
            builder.push(format_args!("\"{}\".\"{}\"", T::TABLE_NAME, e));
        });

        builder.push(" FROM ");
        builder.push(T::TABLE_NAME);
        self.additional_tables.iter().unique().for_each(|e| {
            builder.push(", ");
            builder.push(e);
        });

        if !self.conditions.is_empty() {
            let mut conditions = self.conditions.clone();

            builder.push(" WHERE ");
            if self.conditions.len() == 1 {
                BracketsExpr::new(conditions.pop().unwrap()).push_to(builder);
            } else {
                let left: Box<dyn PushToQuery<T::Database>> =
                    Box::new(BracketsExpr::new(conditions.pop().unwrap()));
                let right: Box<dyn PushToQuery<T::Database>> =
                    Box::new(BracketsExpr::new(conditions.pop().unwrap()));
                let init = BinaryExpr::new(left, right, BinaryExprOperand::And);
                let cond = conditions.into_iter().fold(init, |acc, curr| {
                    BinaryExpr::new(
                        Box::new(acc),
                        Box::new(BracketsExpr::new(curr)),
                        BinaryExprOperand::And,
                    )
                });
                cond.push_to(builder);
            };
        }
    }
}
