use std::{
    fs,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use convert_case::{Case, Casing};
use proc_macro_error2::abort;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::quote;
use sky_orm_sqlparse::schema::{SqlColumn, SqlSchema};
use syn::{
    Attribute, Ident, LitStr, Path, Token,
    parse::{Parse, ParseStream},
    parse2,
    token::Colon,
};

use crate::schema::type_conversion::sql_to_rust_type;

#[derive(Clone)]
struct FieldAddition {
    attrs: Vec<Attribute>,
    ty_override: Option<Path>,
    field_name: Ident,
    rename_to: Option<Ident>,
}

impl Parse for FieldAddition {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let field_name = input.parse::<Ident>()?;

        let rename_to = if input.peek(Token![->]) {
            input.parse::<Token![->]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        let ty_override = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        Ok(Self {
            attrs,
            ty_override,
            field_name,
            rename_to,
        })
    }
}

#[derive(Default)]
struct FieldAdditions(Vec<FieldAddition>);

impl Deref for FieldAdditions {
    type Target = Vec<FieldAddition>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FieldAdditions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Parse for FieldAdditions {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut this = Self::default();

        loop {
            this.push(FieldAddition::parse(input)?);

            if !input.peek(Token![,]) {
                break;
            }

            input.parse::<Token![,]>()?;
        }

        Ok(this)
    }
}

struct DeclModelArgs {
    table_name: LitStr,
    struct_attrs: Vec<Attribute>,
    field_additions: FieldAdditions,
}

impl Parse for DeclModelArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut this = Self {
            struct_attrs: input.call(Attribute::parse_outer)?,
            table_name: input.parse::<LitStr>()?,
            field_additions: FieldAdditions::default(),
        };

        input.parse::<Token![,]>()?;

        while let Ok(ident) = input.parse::<Ident>() {
            input.parse::<Colon>()?;

            let TokenTree::Group(group) = input.parse()? else {
                abort!(input.span(), "Unexpected continuation (expected block)");
            };

            let group_stream = group.stream();

            match ident.to_string().as_str() {
                "fields" => {
                    this.field_additions = parse2::<FieldAdditions>(group_stream)?;
                }
                _ => abort! {
                    ident, "Unknown directive"
                },
            }

            if !input.peek(Token![,]) {
                break;
            }

            input.parse::<Token![,]>()?;
        }

        Ok(this)
    }
}

struct ColumnFieldPairing(SqlColumn, Option<FieldAddition>);

// TODO: refactor with `syn-parse-helpers` to cut down on line length
#[allow(clippy::too_many_lines)]
pub fn decl_model(input: TokenStream) -> TokenStream {
    let arg = match parse2::<DeclModelArgs>(input) {
        Ok(e) => e,
        Err(e) => return e.to_compile_error(),
    };

    let sky_orm_dir: PathBuf = [
        std::env::var("CARGO_MANIFEST_DIR").expect("Missing env var CARGO_MANIFEST_DIR"),
        "sky_orm".to_owned(),
    ]
    .iter()
    .collect();

    let schema_file = fs::read_to_string(sky_orm_dir.join("schema.json"))
        .expect("Failed to read schema.json file");

    let schema: SqlSchema =
        serde_json::from_str(&schema_file).expect("Failed to read schema.json file");

    let table_name = arg.table_name.value();

    let Some(table) = schema.find_table(&table_name) else {
        abort!(
            arg.table_name.span(),
            "Table does not exist in schema. Is it up to date?"
        );
    };

    let field_names = table
        .columns
        .iter()
        .map(|e| e.name.to_case(Case::Snake))
        .collect::<Vec<_>>();

    // Ensure that the specified fields exist, or that a corresponding database column exists for
    // each rename instruction.
    arg.field_additions.iter().for_each(|e| {
        if e.rename_to.is_some() && !table.columns.iter().any(|c| c.name.eq(&e.field_name.to_string())) {
            abort! {
                e.field_name.span(), "Column does not exist in schema.";
                note = "When renaming, make sure to use the exact column name as it appears in the database."
            };
        } else if !field_names.iter().any(|f| f.eq(&e.field_name.to_string())) {
            abort! {
                e.field_name.span(), "Field does not exist on model.";
                note = "Keep in mind that model field names are converted to snake_case!"
            };
        }
    });

    let column_field_pairings = table
        .columns
        .iter()
        .cloned()
        .map(|c| {
            let field_addition = arg.field_additions.iter().find(|e| {
                if e.rename_to.is_some() {
                    c.name.eq(&e.field_name.to_string())
                } else {
                    c.name.to_case(Case::Snake).eq(&e.field_name.to_string())
                }
            });

            ColumnFieldPairing(c, field_addition.cloned())
        })
        .collect::<Vec<_>>();

    let field_quotes = column_field_pairings.iter().map(|e| {
        let (c, field_addition) = (&e.0, e.1.as_ref());

        let field_name = field_addition
            .and_then(|e| e.rename_to.as_ref().map(std::string::ToString::to_string))
            .unwrap_or_else(|| c.name.to_case(Case::Snake));

        let field_name = Ident::new(
            &field_name,
            field_addition.map_or_else(Span::call_site, |e| {
                e.rename_to
                    .as_ref()
                    .map_or(e.field_name.span(), proc_macro2::Ident::span)
            }),
        );

        let attrs = field_addition.map(|e| e.attrs.clone()).unwrap_or_default();

        let ty_quote = field_addition
            .and_then(|e| {
                e.ty_override.as_ref().map(|e| {
                    quote! {
                        #e
                    }
                })
            })
            .unwrap_or_else(|| sql_to_rust_type(&c.column_type));

        let column_name = &c.name;

        quote! {
            #(
                #attrs
            )*
            #[sky_orm(column = #column_name)]
            #field_name: #ty_quote,
        }
    });

    let relation_impls = column_field_pairings.iter().filter_map(|e| {
        e.0.foreign_key.as_ref().map(|foreign_key| {
            let module_name = Ident::new(&foreign_key.target_table, Span::call_site());
            let column_struct_name =
                e.1.as_ref()
                    // need to do this weirdness, because this is how the struct name is computed
                    // inside the derive macro. directly going to PascalCase might have unintended
                    // consequences
                    .map_or_else(|| e.0.name.to_case(Case::Snake).to_case(Case::Pascal),|e| {
                        e.rename_to
                            .as_ref().map_or_else(|| e.field_name.to_string().to_case(Case::Pascal), std::string::ToString::to_string)
                    });

            let column_struct_name = Ident::new(&column_struct_name, Span::call_site());

            let relation_type = if e.0.unique {
                quote! {
                    ::sky_orm::entity::relation::OneToOne
                }
            } else {
                quote! {
                    ::sky_orm::entity::relation::OneToMany
                }
            };

            Some(quote! {
                impl ::sky_orm::relation::Related<super::#module_name::Entity, columns::#column_struct_name, ::sky_orm::sqlx::Postgres> for Entity {
                    type RelationType = #relation_type;
                }
            })
        })
    });

    let sky_orm_attr = if let Some(e) = &table.primary_key {
        let primary_key_field_name = arg
            .field_additions
            .iter()
            .find_map(|f| {
                if f.field_name.to_string().eq(e) {
                    if let Some(r) = &f.rename_to {
                        return Some(r.to_string());
                    }
                }

                None
            })
            .unwrap_or_else(|| e.to_case(Case::Snake));

        quote! {
            #[sky_orm(primary_key = #primary_key_field_name, table = #table_name)]
        }
    } else {
        quote! {
            #[sky_orm(table = #table_name)]
        }
    };

    let struct_attrs = arg.struct_attrs;

    quote! {
        #[derive(::sky_orm::DatabaseModel, ::std::default::Default)]
        #(
            #struct_attrs
        )*
        #sky_orm_attr
        pub struct Model {
            #(
                #field_quotes
            )*
        }

        #(
            #relation_impls
        )*
    }
}
