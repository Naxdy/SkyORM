use darling::{FromDeriveInput, FromField, ast::Data};
use proc_macro_error2::abort;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident, parse2};

#[derive(FromField)]
struct ParseFromRowFieldArgs {
    ident: Option<Ident>,
    column_name: Option<String>,
}

#[derive(FromDeriveInput)]
struct ParseFromRowArgs {
    ident: Ident,
    data: Data<(), ParseFromRowFieldArgs>,
}

pub fn parse_from_row(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse2(input).expect("Failed to parse derive input");

    let args = match ParseFromRowArgs::from_derive_input(&input) {
        Ok(e) => e,
        Err(e) => return e.write_errors(),
    };

    let struct_name = args.ident;

    let Some(struct_args) = args.data.take_struct() else {
        abort! {
            struct_name, "Target is not a struct";
            note = "This macro must be run on a struct.";
        };
    };

    let field_assignments = struct_args.fields.iter().map(|e| {
        let Some(field_name) = &e.ident else {
            abort! {
                e.ident, "Field has no name";
                note = "This macro must not be run on tuple structs";
            };
        };

        let column_name = e
            .column_name
            .clone()
            .unwrap_or_else(|| field_name.to_string());

        quote! {
            #field_name: row.try_get(#column_name)?,
        }
    });

    quote! {
        impl ::sky_orm::query::parse::ParseFromRow for #struct_name {
            fn parse_from_row(row: &::sky_orm::sqlx::any::AnyRow) -> ::std::result::Result<Self, ::sky_orm::sqlx::Error> {
                Ok(Self {
                    #(
                        #field_assignments
                    )*
                })
            }
        }
    }
}
