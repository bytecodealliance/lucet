use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro]
pub fn lucet_integration(args: TokenStream) -> TokenStream {
    let config = parse_macro_input!(args as lucet_wiggle_generate::Config);

    let doc = config.witx.load_document();

    TokenStream::from(lucet_wiggle_generate::generate(&doc, &config))
}
