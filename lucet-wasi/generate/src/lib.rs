use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use wiggle_generate::{define_func, define_module_trait, Names};

mod config;

#[proc_macro]
pub fn bindings(args: TokenStream) -> TokenStream {
    let config = parse_macro_input!(args as config::Config);
    let doc = wasi_common::snapshots::preview_1::metadata::document();
    let names = Names::new(quote!(lucet_wiggle));

    let codegen_settings = wiggle_generate::CodegenSettings::new(
        &config
            .error_conf
            .unwrap_or(wiggle_generate::config::ErrorConf::default()),
        &wiggle_generate::config::AsyncConf::default(),
        &doc,
    )
    .expect("constructing codegen settings");

    let ctx_type = &config.ctx_name;

    let modules = doc.modules().map(|module| {
        let modname = names.module(&module.name);
        let fs = module
            .funcs()
            .map(|f| define_func(&names, &module, &f, &codegen_settings));
        let modtrait = define_module_trait(&names, &module, &codegen_settings);
        quote! {
            pub mod #modname {
                use super::#ctx_type;
                use wasi_common::snapshots::preview_1::types::*;
                #(#fs)*
                #modtrait
            }
        }
    });

    let mut ts = quote! {
        #(#modules)*
    };

    let empty = quote!();
    ts.extend(lucet_wiggle_generate::generate(
        &doc,
        &config.constructor,
        &quote!(wasi_common::snapshots::preview_1),
        config.pre_hook.as_ref().unwrap_or(&empty),
        config.post_hook.as_ref().unwrap_or(&empty),
    ));

    TokenStream::from(ts)
}
