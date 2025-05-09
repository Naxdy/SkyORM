mod model;
mod parse;

use model::derive_database_model;
use proc_macro::TokenStream;
use proc_macro_error2::proc_macro_error;

#[proc_macro_error]
#[proc_macro_derive(DatabaseModel, attributes(sky_orm))]
pub fn model(input: TokenStream) -> TokenStream {
    derive_database_model(input.into()).into()
}

#[proc_macro_error]
#[proc_macro_derive(FromSqlxRow)]
pub fn parse_from_row(input: TokenStream) -> TokenStream {
    parse::parse_from_row(input.into()).into()
}
