extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

#[proc_macro]
pub fn from_witx(args: TokenStream) -> TokenStream {
    let mut config = parse_macro_input!(args as lucet_wiggle_generate::Config);
    config.wiggle.witx.make_paths_relative_to(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANFIEST_DIR env var"),
    );

    let doc = config.wiggle.load_document();

    let names = wiggle_generate::Names::new(&config.wiggle.ctx.name, quote!(lucet_wiggle));
    let error_transform = wiggle_generate::ErrorTransform::new(&config.wiggle.errors, &doc)
        .expect("validating error transform");
    let mut ts = wiggle_generate::generate(&doc, &names, &error_transform);
    ts.extend(wiggle_generate::generate_metadata(&doc, &names));
    ts.extend(lucet_wiggle_generate::generate(
        &doc,
        &config.wiggle.ctx.name,
        &config.constructor,
        &quote!(super),
        &config.pre_hook.unwrap_or(quote!()),
        &config.post_hook.unwrap_or(quote!()),
    ));
    TokenStream::from(ts)
}
