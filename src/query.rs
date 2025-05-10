pub mod parse;
pub mod select;

use std::{fmt::Display, ops::Deref, rc::Rc};

use sqlx::{Any, Encode, QueryBuilder, Type};

/// This trait represents anything that can be pushed into a [`QueryBuilder`], i.e. any kind of
/// query fragment, like a condition or a list of values.
pub trait PushToQuery {
    /// Push the object's contents into a query builder.
    fn push_to(&self, builder: &mut QueryBuilder<'_, Any>);
}

impl PushToQuery for Box<dyn PushToQuery> {
    fn push_to(&self, builder: &mut QueryBuilder<'_, Any>) {
        self.deref().push_to(builder);
    }
}

impl PushToQuery for Rc<dyn PushToQuery> {
    fn push_to(&self, builder: &mut QueryBuilder<'_, Any>) {
        self.deref().push_to(builder);
    }
}

pub(crate) struct QueryVariable<T>(pub(crate) T)
where
    T: for<'a> Encode<'a, Any> + Type<Any> + 'static + Clone;

impl<T> PushToQuery for QueryVariable<T>
where
    T: for<'a> Encode<'a, Any> + Type<Any> + 'static + Clone,
{
    fn push_to(&self, builder: &mut QueryBuilder<'_, Any>) {
        builder.push_bind(self.0.clone());
    }
}

impl<T> PushToQuery for Vec<QueryVariable<T>>
where
    T: for<'a> Encode<'a, Any> + Type<Any> + 'static + Clone,
{
    fn push_to(&self, builder: &mut QueryBuilder<'_, Any>) {
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

pub(crate) struct BracketsExpr<T: PushToQuery>(T);

impl<T: PushToQuery> BracketsExpr<T> {
    pub(crate) fn new(inner: T) -> Self {
        BracketsExpr(inner)
    }
}

impl<T: PushToQuery> PushToQuery for BracketsExpr<T> {
    fn push_to(&self, builder: &mut QueryBuilder<'_, Any>) {
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
                BinaryExprOperand::Gt => ">",
                BinaryExprOperand::Lt => "<",
                BinaryExprOperand::Geq => ">=",
                BinaryExprOperand::Leq => "<=",
            }
        )
    }
}

/// A binary SQL expression, glued together with an operator.
///
/// Example: `left-side [operator] right-side`
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
    pub(crate) fn new(left: T, right: C, operand: BinaryExprOperand) -> Self {
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
    fn push_to(&self, builder: &mut QueryBuilder<'_, Any>) {
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
    fn push_to(&self, builder: &mut QueryBuilder<'_, Any>) {
        self.inner.push_to(builder);
        builder.push(format_args!(" {}", self.operand));
    }
}

impl PushToQuery for String {
    fn push_to(&self, builder: &mut QueryBuilder<'_, Any>) {
        builder.push(self);
    }
}
