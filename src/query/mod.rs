use std::fmt::Display;

use sqlx::{Any, Encode, QueryBuilder, Type};

pub trait PushToQuery {
    fn push_to(self, builder: &mut QueryBuilder<'_, Any>);
}

pub struct QueryVariable<T>(pub(crate) T)
where
    T: for<'a> Encode<'a, Any> + Type<Any> + 'static;

impl<T> PushToQuery for QueryVariable<T>
where
    T: for<'a> Encode<'a, Any> + Type<Any> + 'static,
{
    fn push_to(self, builder: &mut QueryBuilder<'_, Any>) {
        builder.push_bind(self.0);
    }
}

impl<T> PushToQuery for Vec<QueryVariable<T>>
where
    T: for<'a> Encode<'a, Any> + Type<Any> + 'static,
{
    fn push_to(self, builder: &mut QueryBuilder<'_, Any>) {
        builder.push("(");
        self.into_iter().for_each(|e| {
            e.push_to(builder);
        });
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
}

impl Display for BinaryExprOperand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BinaryExprOperand::Equals => "=",
                BinaryExprOperand::DoesNotEqual => "!=",
                BinaryExprOperand::Like => "LIKE",
                BinaryExprOperand::ILike => "ILIKE",
                BinaryExprOperand::And => "AND",
                BinaryExprOperand::Or => "OR",
                BinaryExprOperand::In => "IN",
                BinaryExprOperand::NotIn => "NOT IN",
                BinaryExprOperand::Between => "BETWEEN",
                BinaryExprOperand::NotBetween => "NOT BETWEEN",
            }
        )
    }
}

pub(crate) struct BinaryExpr<T, C>
where
    T: PushToQuery,
    C: PushToQuery,
{
    a: T,
    b: C,
    operand: BinaryExprOperand,
}

impl<T, C> BinaryExpr<T, C>
where
    T: PushToQuery,
    C: PushToQuery,
{
    pub fn new(left: T, right: C, operand: BinaryExprOperand) -> Self {
        Self {
            a: left,
            b: right,
            operand,
        }
    }
}

impl<T, C> PushToQuery for BinaryExpr<T, C>
where
    T: PushToQuery,
    C: PushToQuery,
{
    fn push_to(self, builder: &mut QueryBuilder<'_, Any>) {
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
                SingletonExprOperand::IsNull => "IS NULL",
                SingletonExprOperand::IsNotNull => "IS NOT NULL",
            }
        )
    }
}

pub(crate) struct SingletonExpr<T>
where
    T: PushToQuery,
{
    inner: T,
    operand: SingletonExprOperand,
}

impl<T> SingletonExpr<T>
where
    T: PushToQuery,
{
    pub fn new(inner: T, operand: SingletonExprOperand) -> Self {
        Self { inner, operand }
    }
}

impl<T> PushToQuery for SingletonExpr<T>
where
    T: PushToQuery,
{
    fn push_to(self, builder: &mut QueryBuilder<'_, Any>) {
        self.inner.push_to(builder);
        builder.push(format_args!(" {}", self.operand));
    }
}

impl PushToQuery for String {
    fn push_to(self, builder: &mut QueryBuilder<'_, Any>) {
        builder.push(self);
    }
}
