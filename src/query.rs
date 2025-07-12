pub mod parse;
pub mod select;

use std::{fmt::Display, marker::PhantomData, ops::Deref, sync::Arc};

use sqlx::{Database, Encode, QueryBuilder, Type};

/// This trait represents anything that can be pushed into a [`QueryBuilder`], i.e. any kind of
/// query fragment, like a condition or a list of values.
pub trait PushToQuery<DB>: Send + Sync
where
    DB: Database + Sync,
{
    /// Push the object's contents into a query builder.
    fn push_to(&self, builder: &mut QueryBuilder<'_, DB>);
}

impl<DB> PushToQuery<DB> for Box<dyn PushToQuery<DB>>
where
    DB: Database + Sync,
{
    fn push_to(&self, builder: &mut QueryBuilder<'_, DB>) {
        self.deref().push_to(builder);
    }
}

impl<DB> PushToQuery<DB> for Arc<dyn PushToQuery<DB>>
where
    DB: Database + Sync,
{
    fn push_to(&self, builder: &mut QueryBuilder<'_, DB>) {
        self.deref().push_to(builder);
    }
}

pub(crate) struct QueryVariable<T, DB>(pub(crate) T, PhantomData<DB>)
where
    T: for<'a> Encode<'a, DB> + Type<DB> + 'static + Clone + Send + Sync,
    DB: Database + Sync;

impl<T, DB> QueryVariable<T, DB>
where
    T: for<'a> Encode<'a, DB> + Type<DB> + 'static + Clone + Send + Sync,
    DB: Database + Sync,
{
    pub const fn new(inner: T) -> Self {
        Self(inner, PhantomData)
    }
}

impl<T, DB> PushToQuery<DB> for QueryVariable<T, DB>
where
    T: for<'a> Encode<'a, DB> + Type<DB> + 'static + Clone + Send + Sync,
    DB: Database + Sync,
{
    fn push_to(&self, builder: &mut QueryBuilder<'_, DB>) {
        builder.push_bind(self.0.clone());
    }
}

impl<T, DB> PushToQuery<DB> for Vec<QueryVariable<T, DB>>
where
    T: for<'a> Encode<'a, DB> + Type<DB> + 'static + Clone + Send + Sync,
    DB: Database + Sync,
{
    fn push_to(&self, builder: &mut QueryBuilder<'_, DB>) {
        builder.push("(");
        self.iter().enumerate().for_each(|(i, e)| {
            if i > 0 {
                builder.push(", ");
            }
            e.push_to(builder);
        });
        builder.push(")");
    }
}

pub(crate) struct BracketsExpr<T, DB>(T, PhantomData<DB>)
where
    T: PushToQuery<DB>,
    DB: Database + Sync;

impl<T, DB> BracketsExpr<T, DB>
where
    T: PushToQuery<DB>,
    DB: Database + Sync,
{
    pub(crate) const fn new(inner: T) -> Self {
        Self(inner, PhantomData)
    }
}

impl<T, DB> PushToQuery<DB> for BracketsExpr<T, DB>
where
    T: PushToQuery<DB>,
    DB: Database + Sync,
{
    fn push_to(&self, builder: &mut QueryBuilder<'_, DB>) {
        builder.push("(");
        self.0.push_to(builder);
        builder.push(")");
    }
}

pub(crate) enum BinaryExprOperand {
    Equals,
    DoesNotEqual,
    Like,
    ILike,
    And,
    Or,
    In,
    NotIn,
    Between,
    NotBetween,
    Gt,
    Lt,
    Geq,
    Leq,
}

impl Display for BinaryExprOperand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Equals => "=",
                Self::DoesNotEqual => "!=",
                Self::Like => "LIKE",
                Self::ILike => "ILIKE",
                Self::And => "AND",
                Self::Or => "OR",
                Self::In => "IN",
                Self::NotIn => "NOT IN",
                Self::Between => "BETWEEN",
                Self::NotBetween => "NOT BETWEEN",
                Self::Gt => ">",
                Self::Lt => "<",
                Self::Geq => ">=",
                Self::Leq => "<=",
            }
        )
    }
}

/// A binary SQL expression, glued together with an operator.
///
/// Example: `left-side [operator] right-side`
pub(crate) struct BinaryExpr<T, C, DB>
where
    T: PushToQuery<DB>,
    C: PushToQuery<DB>,
    DB: Database + Sync,
{
    a: T,
    b: C,
    operand: BinaryExprOperand,
    marker: PhantomData<DB>,
}

impl<T, C, DB> BinaryExpr<T, C, DB>
where
    T: PushToQuery<DB>,
    C: PushToQuery<DB>,
    DB: Database + Sync,
{
    pub(crate) const fn new(left: T, right: C, operand: BinaryExprOperand) -> Self {
        Self {
            a: left,
            b: right,
            operand,
            marker: PhantomData,
        }
    }
}

impl<T, C, DB> PushToQuery<DB> for BinaryExpr<T, C, DB>
where
    T: PushToQuery<DB>,
    C: PushToQuery<DB>,
    DB: Database + Sync,
{
    fn push_to(&self, builder: &mut QueryBuilder<'_, DB>) {
        self.a.push_to(builder);
        builder.push(format_args!(" {} ", self.operand));
        self.b.push_to(builder);
    }
}

pub(crate) enum SingletonExprOperand {
    IsNull,
    IsNotNull,
}

impl Display for SingletonExprOperand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::IsNull => "IS NULL",
                Self::IsNotNull => "IS NOT NULL",
            }
        )
    }
}

pub(crate) struct SingletonExpr<T, DB>
where
    T: PushToQuery<DB>,
    DB: Database + Sync,
{
    inner: T,
    operand: SingletonExprOperand,
    marker: PhantomData<DB>,
}

impl<T, DB> SingletonExpr<T, DB>
where
    T: PushToQuery<DB>,
    DB: Database + Sync,
{
    pub const fn new(inner: T, operand: SingletonExprOperand) -> Self {
        Self {
            inner,
            operand,
            marker: PhantomData,
        }
    }
}

impl<T, DB> PushToQuery<DB> for SingletonExpr<T, DB>
where
    T: PushToQuery<DB>,
    DB: Database + Sync,
{
    fn push_to(&self, builder: &mut QueryBuilder<'_, DB>) {
        self.inner.push_to(builder);
        builder.push(format_args!(" {}", self.operand));
    }
}

impl<DB> PushToQuery<DB> for String
where
    DB: Database + Sync,
{
    fn push_to(&self, builder: &mut QueryBuilder<'_, DB>) {
        builder.push(self);
    }
}
