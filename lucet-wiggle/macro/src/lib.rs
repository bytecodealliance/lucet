use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

#[proc_macro]
pub fn from_witx(args: TokenStream) -> TokenStream {
    let config = parse_macro_input!(args as lucet_wiggle_generate::Config);

    let doc = config.wiggle.load_document();

    let names = wiggle_generate::Names::new(quote!(lucet_wiggle));
    let codegen_settings =
        wiggle_generate::CodegenSettings::new(&config.wiggle.errors, &config.wiggle.async_, &doc)
            .expect("validating error transform");
    let mut ts = wiggle_generate::generate(&doc, &names, &codegen_settings);
    ts.extend(wiggle_generate::generate_metadata(&doc, &names));
    ts.extend(lucet_wiggle_generate::generate(
        &doc,
        &config.constructor,
        &quote!(super),
        &config.pre_hook.unwrap_or_default(),
        &config.post_hook.unwrap_or_default(),
        &codegen_settings,
    ));
    TokenStream::from(ts)
}
