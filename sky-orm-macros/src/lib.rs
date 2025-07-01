mod model;
mod parse;
mod schema;

use model::derive_database_model;
use proc_macro::TokenStream;
use proc_macro_error2::proc_macro_error;

#[proc_macro_error]
#[proc_macro_derive(DatabaseModel, attributes(sky_orm))]
pub fn database_model(input: TokenStream) -> TokenStream {
    derive_database_model(input.into()).into()
}

#[proc_macro_error]
#[proc_macro_derive(FromSqlxRow)]
pub fn parse_from_row(input: TokenStream) -> TokenStream {
    parse::parse_from_row(input.into()).into()
}

#[proc_macro_error]
#[proc_macro]
pub fn model(input: TokenStream) -> TokenStream {
    schema::model::decl_model(input.into()).into()
}
