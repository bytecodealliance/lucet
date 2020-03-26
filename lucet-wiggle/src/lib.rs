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

    let doc = witx::load(&config.wiggle.witx.paths).expect("lucet loading witx");

    let mut ts = wiggle_generate::generate(&doc, &config.wiggle);
    ts.extend(lucet_wiggle_generate::generate(
        &doc,
        &config.wiggle.ctx.name,
        &config.constructor,
        &quote!(self),
    ));

    TokenStream::from(ts)
}
