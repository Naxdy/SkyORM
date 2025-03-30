use super::{
    Entity,
    column::{Column, ComparableColumn},
};

pub enum RelationType {
    OneToOne,
    ManyToOne,
}

pub enum InverseRelationType {
    OneToOne,
    OneToMany,
}

/// The owning side (= the side with the foreign key stored in its table) of a database relation.
///
/// Implementing this trait will automatically implement [`InverseRelated`] for the opposite side.
pub trait Related<R>: Entity
where
    R: Entity,
{
    /// The column holding the foreign key to the other entity's primary key.
    fn fk_column()
    -> impl ComparableColumn<Entity = Self, Type = <R::PrimaryKeyColumn as Column>::Type>;

    /// The relation type, i.e. how many other entities are expected to be on the other side.
    fn relation_type() -> RelationType;
}

/// The non-owning or inverse side of a database relation.
///
/// This trait is auto implemented for the opposite sides whenever [`Related`] is implemented.
pub trait InverseRelated<R>: Entity
where
    R: Entity,
{
    /// The column on the other entity holding the foreign key to this entity's primary key.
    fn inverse_fk_column()
    -> impl ComparableColumn<Entity = R, Type = <Self::PrimaryKeyColumn as Column>::Type>;

    /// The relation type, i.e. how many other entities are expected to be on the other side.
    ///
    /// Note that this is meant to be from the perspective of _this_ entity, so if the other entity
    /// has a ManyToOne relation, this would be a OneToMany relation.
    fn inverse_relation_type() -> InverseRelationType;
}

impl<E, R> InverseRelated<E> for R
where
    E: Related<R>,
    R: Entity,
{
    fn inverse_fk_column()
    -> impl ComparableColumn<Entity = E, Type = <Self::PrimaryKeyColumn as Column>::Type> {
        E::fk_column()
    }

    fn inverse_relation_type() -> InverseRelationType {
        match E::relation_type() {
            RelationType::OneToOne => InverseRelationType::OneToOne,
            RelationType::ManyToOne => InverseRelationType::OneToMany,
        }
    }
}
