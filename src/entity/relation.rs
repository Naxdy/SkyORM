use sealed::Sealed;
use sqlx::{Connection, Database, Executor, IntoArguments, Result};

use crate::entity::model::{GetColumn, Model};

use super::{
    Entity,
    column::{Column, ComparableColumn},
};

/// A one-to-one relation.
#[derive(Default, Clone, Copy)]
pub struct OneToOne;

/// A many-to-one relation (owning side).
#[derive(Default, Clone, Copy)]
pub struct ManyToOne;

/// A one-to-many relation (non-owning side).
#[derive(Default, Clone, Copy)]
pub struct OneToMany;

impl Relation for OneToOne {
    type InverseEquivalent = Self;
}

impl InverseRelation for OneToOne {
    type ForwardEquivalent = Self;
}

impl Relation for ManyToOne {
    type InverseEquivalent = OneToMany;
}

impl InverseRelation for OneToMany {
    type ForwardEquivalent = ManyToOne;
}

/// Trait defining the owning side of a relation.
///
/// Sealed trait, not meant for manual implementation.
pub trait Relation: Sealed {
    type InverseEquivalent: InverseRelation;
}

/// Trait defining the non-owning side of a relation.
///
/// Sealed trait, not meant for manual implementation.
pub trait InverseRelation: Sealed {
    type ForwardEquivalent: Relation;
}

/// The owning side (= the side with the foreign key stored in its table) of a database relation.
/// `C` is the column holding the foreign key to the other entity's primary key.
///
/// Implementing this trait will automatically implement [`InverseRelated`] for the other side.
pub trait Related<R, C>: Entity
where
    R: Entity<Database = Self::Database>,
    C: ComparableColumn<Entity = Self, Type = <R::PrimaryKeyColumn as Column>::Type>,
{
    /// The relation type, i.e. how many other entities are expected to be on the other side.
    type RelationType: Relation;
}

/// The non-owning or inverse side of a database relation.
/// `C` is the column on the other entity holding the foreign key to this entity's primary key.
///
/// This trait is auto implemented for the opposing sides whenever [`Related`] is implemented.
pub trait InverseRelated<R, C>: Entity
where
    R: Entity<Database = Self::Database>,
    C: ComparableColumn<Entity = R, Type = <Self::PrimaryKeyColumn as Column>::Type>,
{
    /// The relation type, i.e. how many other entities are expected to be on the other side.
    ///
    /// Note that this is meant to be from the perspective of _this_ entity, so if the other entity
    /// has a ManyToOne relation, this would be a OneToMany relation.
    type InverseRelationType: InverseRelation;
}

impl<E, R, C> InverseRelated<E, C> for R
where
    E: Related<R, C, Database = R::Database>,
    R: Entity,
    C: ComparableColumn<Entity = E, Type = <Self::PrimaryKeyColumn as Column>::Type>,
{
    type InverseRelationType = <E::RelationType as Relation>::InverseEquivalent;
}

pub trait LoadRelation<T, C, R, O>
where
    T: Model + GetColumn<<T::Entity as Entity>::PrimaryKeyColumn> + 'static,
    R: Related<T::Entity, C> + Entity<Database = <T::Entity as Entity>::Database> + Send + 'static,
    R::Model: GetColumn<C> + Clone,
    C: Column
        + ComparableColumn<
            Entity = R,
            Type = <<T::Entity as Entity>::PrimaryKeyColumn as Column>::Type,
        > + 'static,
    T::Entity: Entity<Model = T>,
    <T::Entity as Entity>::PrimaryKeyColumn: Clone,
    <<T::Entity as Entity>::PrimaryKeyColumn as Column>::Type: PartialEq,
{
    fn load_relation<'c, Conn>(self, connection: &'c mut Conn) -> impl Future<Output = Result<O>>
    where
        Conn: Connection<Database = R::Database>,
        &'c mut Conn: Executor<'c, Database = R::Database>,
        for<'q> <R::Database as Database>::Arguments<'q>: IntoArguments<'q, R::Database> + 'c;
}

// TODO: add non-nullable variants

impl<T, C, R> LoadRelation<T, C, R, Vec<Option<T>>> for &[R::Model]
where
    T: Model + GetColumn<<T::Entity as Entity>::PrimaryKeyColumn> + Clone + 'static,
    R: Related<T::Entity, C> + Entity<Database = <T::Entity as Entity>::Database> + Send + 'static,
    R::Model: GetColumn<C> + Clone,
    C: Column
        + ComparableColumn<
            Entity = R,
            Type = <<T::Entity as Entity>::PrimaryKeyColumn as Column>::Type,
        > + 'static,
    T::Entity: Entity<Model = T>,
    <T::Entity as Entity>::PrimaryKeyColumn: Clone,
    <<T::Entity as Entity>::PrimaryKeyColumn as Column>::Type: PartialEq,
{
    async fn load_relation<'c, Conn>(self, connection: &'c mut Conn) -> Result<Vec<Option<T>>>
    where
        Conn: Connection<Database = R::Database>,
        &'c mut Conn: Executor<'c, Database = R::Database>,
        for<'q> <R::Database as Database>::Arguments<'q>: IntoArguments<'q, R::Database> + 'c,
    {
        let results = <T::Entity as Entity>::find()
            .filter(<T::Entity as Entity>::PrimaryKeyColumn::is_in(
                &self.iter().map(|e| e.get().clone()).collect::<Vec<_>>(),
            ))
            .all(connection)
            .await?;

        Ok(self
            .iter()
            .map(|e| results.iter().find(|r| r.get() == e.get()).cloned())
            .collect())
    }
}

impl<T, C, R> LoadRelation<T, C, R, Option<T>> for &R::Model
where
    T: Model + GetColumn<<T::Entity as Entity>::PrimaryKeyColumn> + Clone + 'static,
    R: Related<T::Entity, C> + Entity<Database = <T::Entity as Entity>::Database> + Send + 'static,
    R::Model: GetColumn<C> + Clone,
    C: Column
        + ComparableColumn<
            Entity = R,
            Type = <<T::Entity as Entity>::PrimaryKeyColumn as Column>::Type,
        > + 'static,
    T::Entity: Entity<Model = T>,
    <T::Entity as Entity>::PrimaryKeyColumn: Clone,
    <<T::Entity as Entity>::PrimaryKeyColumn as Column>::Type: PartialEq,
{
    async fn load_relation<'c, Conn>(self, connection: &'c mut Conn) -> Result<Option<T>>
    where
        Conn: Connection<Database = R::Database>,
        &'c mut Conn: Executor<'c, Database = R::Database>,
        for<'q> <R::Database as Database>::Arguments<'q>: IntoArguments<'q, R::Database> + 'c,
    {
        let result = <T::Entity as Entity>::find()
            .filter(<T::Entity as Entity>::PrimaryKeyColumn::eq(
                self.get().clone(),
            ))
            .one(connection)
            .await;

        if let Err(sqlx::Error::RowNotFound) = result {
            Ok(None)
        } else {
            Ok(Some(result?))
        }
    }
}

pub trait LoadInverse<T, C, R, O>
where
    T: Model + GetColumn<<T::Entity as Entity>::PrimaryKeyColumn> + 'static,
    R: Related<T::Entity, C> + Entity<Database = <T::Entity as Entity>::Database> + Send + 'static,
    R::Model: GetColumn<C> + Clone,
    C: Column
        + ComparableColumn<
            Entity = R,
            Type = <<T::Entity as Entity>::PrimaryKeyColumn as Column>::Type,
        > + 'static,
    <T::Entity as Entity>::PrimaryKeyColumn: Clone,
    <<T::Entity as Entity>::PrimaryKeyColumn as Column>::Type: PartialEq,
{
    fn load_inverse<'c, Conn>(self, connection: &'c mut Conn) -> impl Future<Output = Result<O>>
    where
        Conn: Connection<Database = R::Database>,
        &'c mut Conn: Executor<'c, Database = R::Database>,
        for<'q> <R::Database as Database>::Arguments<'q>: IntoArguments<'q, R::Database> + 'c;
}

impl<T, C, R> LoadInverse<T, C, R, Vec<Option<R::Model>>> for &[T]
where
    T: Model + GetColumn<<T::Entity as Entity>::PrimaryKeyColumn> + 'static,
    R: Related<T::Entity, C, RelationType = OneToOne>
        + Entity<Database = <T::Entity as Entity>::Database>
        + Send
        + 'static,
    R::Model: GetColumn<C> + Clone,
    C: Column
        + ComparableColumn<
            Entity = R,
            Type = <<T::Entity as Entity>::PrimaryKeyColumn as Column>::Type,
        > + 'static,
    <T::Entity as Entity>::PrimaryKeyColumn: Clone,
    <<T::Entity as Entity>::PrimaryKeyColumn as Column>::Type: PartialEq,
{
    async fn load_inverse<'c, Conn>(self, connection: &'c mut Conn) -> Result<Vec<Option<R::Model>>>
    where
        Conn: Connection<Database = R::Database>,
        &'c mut Conn: Executor<'c, Database = R::Database>,
        for<'q> <R::Database as Database>::Arguments<'q>: IntoArguments<'q, R::Database> + 'c,
    {
        let results = R::find()
            .filter(C::is_in(
                &self.iter().map(|e| e.get().clone()).collect::<Vec<_>>(),
            ))
            .all(connection)
            .await?;

        Ok(self
            .iter()
            .map(|e| results.iter().find(|r| r.get() == e.get()).cloned())
            .collect())
    }
}

impl<T, C, R> LoadInverse<T, C, R, Vec<Vec<R::Model>>> for &[T]
where
    T: Model + GetColumn<<T::Entity as Entity>::PrimaryKeyColumn> + 'static,
    R: Related<T::Entity, C, RelationType = ManyToOne>
        + Entity<Database = <T::Entity as Entity>::Database>
        + Send
        + 'static,
    R::Model: GetColumn<C> + Clone,
    C: Column
        + ComparableColumn<
            Entity = R,
            Type = <<T::Entity as Entity>::PrimaryKeyColumn as Column>::Type,
        > + 'static,
    <T::Entity as Entity>::PrimaryKeyColumn: Clone,
    <<T::Entity as Entity>::PrimaryKeyColumn as Column>::Type: PartialEq,
{
    async fn load_inverse<'c, Conn>(self, connection: &'c mut Conn) -> Result<Vec<Vec<R::Model>>>
    where
        Conn: Connection<Database = R::Database>,
        &'c mut Conn: Executor<'c, Database = R::Database>,
        for<'q> <R::Database as Database>::Arguments<'q>: IntoArguments<'q, R::Database> + 'c,
    {
        let results = R::find()
            .filter(C::is_in(
                &self.iter().map(|e| e.get().clone()).collect::<Vec<_>>(),
            ))
            .all(connection)
            .await?;

        Ok(self
            .iter()
            .map(|e| {
                results
                    .iter()
                    .filter(|r| r.get() == e.get())
                    .cloned()
                    .collect()
            })
            .collect())
    }
}

impl<T, C, R> LoadInverse<T, C, R, Option<R::Model>> for &T
where
    T: Model + GetColumn<<T::Entity as Entity>::PrimaryKeyColumn> + 'static,
    R: Related<T::Entity, C, RelationType = OneToOne>
        + Entity<Database = <T::Entity as Entity>::Database>
        + Send
        + 'static,
    R::Model: GetColumn<C> + Clone,
    C: Column
        + ComparableColumn<
            Entity = R,
            Type = <<T::Entity as Entity>::PrimaryKeyColumn as Column>::Type,
        > + 'static,
    <T::Entity as Entity>::PrimaryKeyColumn: Clone,
    <<T::Entity as Entity>::PrimaryKeyColumn as Column>::Type: PartialEq,
{
    async fn load_inverse<'c, Conn>(self, connection: &'c mut Conn) -> Result<Option<R::Model>>
    where
        Conn: Connection<Database = R::Database>,
        &'c mut Conn: Executor<'c, Database = R::Database>,
        for<'q> <R::Database as Database>::Arguments<'q>: IntoArguments<'q, R::Database> + 'c,
    {
        let result = R::find()
            .filter(C::eq(self.get().clone()))
            .one(connection)
            .await;

        if let Err(sqlx::Error::RowNotFound) = result {
            Ok(None)
        } else {
            Ok(Some(result?))
        }
    }
}

impl<T, C, R> LoadInverse<T, C, R, Vec<R::Model>> for &T
where
    T: Model + GetColumn<<T::Entity as Entity>::PrimaryKeyColumn> + 'static,
    R: Related<T::Entity, C, RelationType = ManyToOne>
        + Entity<Database = <T::Entity as Entity>::Database>
        + Send
        + 'static,
    R::Model: GetColumn<C> + Clone,
    C: Column
        + ComparableColumn<
            Entity = R,
            Type = <<T::Entity as Entity>::PrimaryKeyColumn as Column>::Type,
        > + 'static,
    <T::Entity as Entity>::PrimaryKeyColumn: Clone,
    <<T::Entity as Entity>::PrimaryKeyColumn as Column>::Type: PartialEq,
{
    async fn load_inverse<'c, Conn>(self, connection: &'c mut Conn) -> Result<Vec<R::Model>>
    where
        Conn: Connection<Database = R::Database>,
        &'c mut Conn: Executor<'c, Database = R::Database>,
        for<'q> <R::Database as Database>::Arguments<'q>: IntoArguments<'q, R::Database> + 'c,
    {
        R::find()
            .filter(C::eq(self.get().clone()))
            .all(connection)
            .await
    }
}

mod sealed {
    use super::{ManyToOne, OneToMany, OneToOne};

    pub trait Sealed {}

    impl Sealed for OneToOne {}
    impl Sealed for OneToMany {}
    impl Sealed for ManyToOne {}
}
