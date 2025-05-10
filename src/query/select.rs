use std::{marker::PhantomData, rc::Rc};

use futures::{StreamExt, stream::FuturesUnordered};
use itertools::Itertools;
use sqlx::{AnyConnection, Executor, QueryBuilder};

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
    conditions: Vec<Rc<dyn PushToQuery>>,
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
    pub fn filter<Q>(mut self, condition: EntityConditionExpr<Q, T>) -> Self
    where
        Q: PushToQuery + 'static,
    {
        self.conditions.push(Rc::new(condition));
        self
    }

    /// Append a new `WHERE` condition using an `AND` statement as glue, allowing to filter the
    /// columns of a related entity (the foreign key is on `R`). The passed condition is wrapped
    /// in `()` brackets.
    pub fn where_relation<Q, R>(mut self, condition: EntityConditionExpr<Q, R>) -> Self
    where
        Q: PushToQuery + 'static,
        R: Related<T> + 'static,
        T: InverseRelated<R>,
    {
        self.conditions.push(Rc::new(condition));
        self.conditions.push(Rc::new(BinaryExpr::new(
            <R::FkColumn as Column>::full_column_name(),
            <T::PrimaryKeyColumn as Column>::full_column_name(),
            BinaryExprOperand::Equals,
        )));
        self.additional_tables.push(R::TABLE_NAME.to_string());
        self
    }

    /// Append a new `WHERE` condition using an `AND` statement as glue, allowing to filter the
    /// columns of an inversely related entity (the foreign key is on `T`). The passed condition is
    /// wrapped in `()` brackets.
    pub fn where_inverse_relation<Q, R>(mut self, condition: EntityConditionExpr<Q, R>) -> Self
    where
        Q: PushToQuery + 'static,
        R: InverseRelated<T> + 'static,
        T: Related<R>,
    {
        self.conditions.push(Rc::new(condition));
        self.conditions.push(Rc::new(BinaryExpr::new(
            <<T as Related<R>>::FkColumn as Column>::full_column_name(),
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
    pub fn query(&self) -> String {
        let mut builder = QueryBuilder::new("");
        self.push_to(&mut builder);
        builder.into_sql()
    }

    /// Execute the query, returning a single result.
    pub async fn one(&self, connection: &mut AnyConnection) -> Result<T::Model, sqlx::Error> {
        let mut builder = QueryBuilder::new("");
        self.push_to(&mut builder);
        let result = connection.fetch_one(builder.build()).await?;
        <T::Model as ParseFromRow>::parse_from_row(&result)
    }

    /// Execute the query, returning all results.
    pub async fn many(&self, connection: &mut AnyConnection) -> Result<Vec<T::Model>, sqlx::Error> {
        let mut builder = QueryBuilder::new("");
        self.push_to(&mut builder);

        let result = connection
            .fetch(builder.build())
            // TODO: check if this messes with `ORDER BY`
            .collect::<FuturesUnordered<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;

        result
            .iter()
            .map(<T::Model as ParseFromRow>::parse_from_row)
            .collect::<Result<Vec<_>, _>>()
    }
}

impl<T> PushToQuery for Select<T>
where
    T: Entity + 'static,
{
    fn push_to(&self, builder: &mut sqlx::QueryBuilder<'_, sqlx::Any>) {
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
                let left: Box<dyn PushToQuery> =
                    Box::new(BracketsExpr::new(conditions.pop().unwrap()));
                let right: Box<dyn PushToQuery> =
                    Box::new(BracketsExpr::new(conditions.pop().unwrap()));
                let init = BinaryExpr::new(left, right, BinaryExprOperand::And);
                let cond = conditions
                    .drain(0..self.conditions.len())
                    .fold(init, |acc, curr| {
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
