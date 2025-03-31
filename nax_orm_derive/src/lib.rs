mod model;

use model::derive_database_model;
use proc_macro::TokenStream;
use proc_macro_error2::proc_macro_error;

#[proc_macro_error]
#[proc_macro_derive(DatabaseModel, attributes(nax_orm))]
pub fn model(input: TokenStream) -> TokenStream {
    derive_database_model(input)
}
