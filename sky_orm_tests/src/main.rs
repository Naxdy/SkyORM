use sky_orm::entity::{
    Entity,
    column::{ComparableColumn, OrderableColumn},
};

mod my_entity {
    use sky_orm::entity::relation::{OneToOne, Related};
    use sky_orm_derive::DatabaseModel;

    #[derive(DatabaseModel)]
    #[sky_orm(primary_key = id, table = "entity")]
    pub struct Model {
        id: String,
        name: Option<String>,
        other_entity_id: String,
    }

    impl Related<super::my_other_entity::Entity> for Entity {
        type FkColumn = columns::OtherEntityId;
        type RelationType = OneToOne;
    }
}

mod my_other_entity {
    use sky_orm_derive::DatabaseModel;

    #[derive(DatabaseModel, Default)]
    #[sky_orm(primary_key = id, table = "other_entity")]
    pub struct Model {
        pub id: String,
        pub amount_killed: i32,
        pub other_amount_killed: i32,
    }
}

fn main() {
    let q = my_entity::Entity::find()
        .filter(my_entity::columns::Name::between(
            Some("August".to_string()),
            Some("Gustav".to_string()),
        ))
        .where_inverse_relation(my_other_entity::columns::AmountKilled::gt(5));

    let oq = my_other_entity::Entity::find()
        .filter(
            my_other_entity::columns::AmountKilled::lt(69)
                .or(my_other_entity::columns::OtherAmountKilled::gt(51)),
        )
        .filter(my_other_entity::columns::AmountKilled::is_not_in([
            0, 1, 2, 3, 4,
        ]))
        .where_relation(my_entity::columns::Name::eq(Some(
            "August Heinrich".to_string(),
        )));

    println!("Q: {}", q.query());
    println!("OQ: {}", oq.query());
}
