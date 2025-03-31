use nax_orm::entity::{Entity, column::Column, model::Model};

mod my_entity {
    use nax_orm::entity::relation::{Related, RelationType};
    use nax_orm_derive::DatabaseModel;

    #[derive(DatabaseModel)]
    #[nax_orm(primary_key = id)]
    pub struct Model {
        id: String,
        name: Option<String>,
        other_entity_id: String,
    }

    impl Related<super::my_other_entity::Entity> for Entity {
        fn fk_column()
        -> impl nax_orm::entity::column::ComparableColumn<Entity = Self, Type = <<super::my_other_entity::Entity as nax_orm::entity::Entity>::PrimaryKeyColumn as nax_orm::entity::column::Column>::Type>{
            columns::OtherEntityId
        }

        fn relation_type() -> nax_orm::entity::relation::RelationType {
            RelationType::OneToOne
        }
    }
}

mod my_other_entity {
    use nax_orm_derive::DatabaseModel;

    #[derive(Default)]
    pub struct NewType(String);

    #[derive(DatabaseModel, Default)]
    #[nax_orm(primary_key = id)]
    pub struct Model {
        pub id: String,
        pub amount_killed: i32,
        pub other_amount_killed: i32,
    }
}

fn main() {
    let model = my_other_entity::Model::default();

    let model = model.into_active();

    println!("{:?}", my_other_entity::Entity::COLUMN_NAMES);
}
