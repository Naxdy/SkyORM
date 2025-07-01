use convert_case::{Case, Casing};
use darling::{FromDeriveInput, FromField, ast::Data};
use proc_macro_error2::{abort, emit_error};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident, Type, Visibility, parse2};

#[derive(FromField, Debug, Clone)]
#[darling(attributes(sky_orm))]
struct DeriveModelField {
    ident: Option<Ident>,
    ty: Type,
    column: Option<String>,
    vis: Visibility,
}

#[derive(FromDeriveInput)]
#[darling(attributes(sky_orm))]
struct DeriveModelTarget {
    ident: Ident,
    table: Option<String>,
    primary_key: Ident,
    data: Data<(), DeriveModelField>,
}

#[derive(Clone)]
struct TargetColumn {
    field_ident: Ident,
    db_name: String,
    struct_name: String,
    ty: Type,
    field_vis: Visibility,
}

pub fn derive_database_model(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse2(input).expect("Failed to parse derive input");

    let target = match DeriveModelTarget::from_derive_input(&input) {
        Ok(r) => r,
        Err(e) => return e.write_errors(),
    };

    let Some(struct_data) = target.data.take_struct() else {
        abort! {
            input, "Target is not a struct.";
            note = "This macro must be run on a struct.";
        };
    };

    let columns = struct_data
        .fields
        .iter()
        .map(|e| {
            let Some(ident) = &e.ident else {
                abort! {
                    e.ident, "Field has no ident.";
                    note = "This macro cannot be run on tuple structs.";
                };
            };

            TargetColumn {
                field_ident: ident.clone(),
                db_name: e.column.as_ref().cloned().unwrap_or(ident.to_string()),
                struct_name: ident.to_string().to_case(Case::Pascal),
                ty: e.ty.clone(),
                field_vis: e.vis.clone(),
            }
        })
        .collect::<Vec<_>>();

    // Make sure all columns have unique names.
    if let Some(duplicate) = columns
        .iter()
        .find(|e| columns.iter().filter(|o| e.db_name.eq(&o.db_name)).count() > 1)
    {
        columns.iter().for_each(|e| {
            if columns.iter().filter(|o| e.db_name.eq(&o.db_name)).count() > 1 {
                emit_error! {
                    e.field_ident.span(), "Clashing occurrence of \"{}\" here.", e.db_name
                };
            }
        });

        abort! {
            duplicate.field_ident.span(), "Duplicate column definition \"{}\"", duplicate.db_name;
            note = "Columns must have unique names, if necessary use the #[sky_orm(column = \"my_column_name\")] attribute to specify a unique name.";
        }
    }

    let Some(primary_key_struct_ident) = columns.iter().find_map(|e| {
        if e.field_ident.eq(&target.primary_key) {
            Some(Ident::new(e.struct_name.as_str(), e.field_ident.span()))
        } else {
            None
        }
    }) else {
        abort! {
            input, "Missing primary key.";
            note = "You need to specify which column is supposed to act as the primary key, using #[sky_orm(primary_key = field_name)]";
        }
    };

    let columns_module = {
        let column_impls = columns.iter().map(|e| {
            let struct_name = Ident::new(e.struct_name.as_str(), e.field_ident.span());
            let db_name = &e.db_name;
            let ty = &e.ty;

            quote! {
                pub struct #struct_name;

                impl ::sky_orm::entity::column::Column for #struct_name {
                    type Type = #ty;
                    type Entity = super::Entity;
                    const NAME: &'static str = #db_name;
                }
            }
        });

        quote! {
            pub mod columns {
                #(
                    #column_impls
                )*
            }
        }
    };

    let model_ident = &target.ident;

    let entity_impl = {
        let table_name = target
            .table
            .unwrap_or(target.ident.to_string().to_case(Case::Snake));

        let column_names_decl = columns.iter().map(|e| &e.db_name);

        quote! {
            pub struct Entity;

            impl ::sky_orm::entity::Entity for Entity {
                type PrimaryKeyColumn = columns::#primary_key_struct_ident;

                type Model = #model_ident;

                type Database = ::sky_orm::sqlx::Postgres;

                const TABLE_NAME: &'static str = #table_name;

                const COLUMN_NAMES: &[&'static str] = &[
                    #(#column_names_decl),*
                ];
            }
        }
    };

    let model_impl = {
        let column_field_assignments = columns.iter().map(|e| {
            let field_ident = &e.field_ident;
            let column_struct_name = Ident::new(e.struct_name.as_str(), field_ident.span());

            quote! {
                #field_ident: columns::#column_struct_name::value_from_row(row)?,
            }
        });

        let active_model_field_assignments = columns.iter().map(|e| {
            let ident = &e.field_ident;

            quote! {
                #ident: ::sky_orm::entity::model::ActiveModelValue::Unchanged(self.#ident),
            }
        });

        quote! {
            impl ::sky_orm::entity::model::Model for #model_ident {
                type Entity = Entity;
                type ActiveModel = ActiveModel;

                fn into_active(self) -> Self::ActiveModel {
                    ActiveModel {
                        #(
                            #active_model_field_assignments
                        )*
                    }
                }
            }

            impl ::sky_orm::query::parse::ParseFromRow<::sky_orm::sqlx::Postgres> for #model_ident {
                fn parse_from_row(row: &<::sky_orm::sqlx::Postgres as ::sky_orm::sqlx::Database>::Row) -> ::std::result::Result<Self, ::sky_orm::sqlx::Error>
                where
                    for<'a> &'a str: ::sky_orm::sqlx::ColumnIndex<<::sky_orm::sqlx::Postgres as ::sky_orm::sqlx::Database>::Row>,
                {
                    use ::sky_orm::entity::column::Column;

                    Ok(Self {
                        #(
                            #column_field_assignments
                        )*
                    })
                }
            }
        }
    };

    let active_model_impl = {
        let active_model_field_decls = columns.iter().map(|e| {
            let ident = &e.field_ident;
            let ty = &e.ty;
            let vis = &e.field_vis;

            quote! {
                #vis #ident: ::sky_orm::entity::model::ActiveModelValue<#ty, ::sky_orm::sqlx::Postgres>,
            }
        });

        quote! {
            pub struct ActiveModel {
                #(
                    #active_model_field_decls
                )*
            }

            impl ::sky_orm::entity::model::ActiveModel for ActiveModel {
                type Model = #model_ident;
            }
        }
    };

    quote! {
        #model_impl

        #active_model_impl

        #entity_impl

        #columns_module
    }
}
