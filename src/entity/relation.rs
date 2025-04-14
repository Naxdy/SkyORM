use sealed::Sealed;

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
///
/// Implementing this trait will automatically implement [`InverseRelated`] for the other side.
pub trait Related<R>: Entity
where
    R: Entity,
{
    /// The column holding the foreign key to the other entity's primary key.
    type FkColumn: ComparableColumn<Entity = Self, Type = <R::PrimaryKeyColumn as Column>::Type>;

    /// The relation type, i.e. how many other entities are expected to be on the other side.
    type RelationType: Relation;
}

/// The non-owning or inverse side of a database relation.
///
/// This trait is auto implemented for the opposing sides whenever [`Related`] is implemented.
pub trait InverseRelated<R>: Entity
where
    R: Entity,
{
    /// The column on the other entity holding the foreign key to this entity's primary key.
    type InverseFkColumn: ComparableColumn<Entity = R, Type = <Self::PrimaryKeyColumn as Column>::Type>;

    /// The relation type, i.e. how many other entities are expected to be on the other side.
    ///
    /// Note that this is meant to be from the perspective of _this_ entity, so if the other entity
    /// has a ManyToOne relation, this would be a OneToMany relation.
    type InverseRelationType: InverseRelation;
}

impl<E, R> InverseRelated<E> for R
where
    E: Related<R>,
    R: Entity,
{
    type InverseFkColumn = E::FkColumn;
    type InverseRelationType = <E::RelationType as Relation>::InverseEquivalent;
}

mod sealed {
    use super::{ManyToOne, OneToMany, OneToOne};

    pub trait Sealed {}

    impl Sealed for OneToOne {}
    impl Sealed for OneToMany {}
    impl Sealed for ManyToOne {}
}
