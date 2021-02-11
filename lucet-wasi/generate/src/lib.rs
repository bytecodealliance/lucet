use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use wiggle_generate::{define_func, define_module_trait, Names};

mod config;

#[proc_macro]
pub fn bindings(args: TokenStream) -> TokenStream {
    let config = parse_macro_input!(args as config::Config);
    let doc = wasi_common::snapshots::preview_1::metadata::document();
    let names = Names::new(&config.ctx_name, quote!(lucet_wiggle));

    let error_transform = if let Some(error_conf) = config.error_conf {
        wiggle_generate::ErrorTransform::new(&error_conf, &doc)
            .expect("constructing error transform")
    } else {
        wiggle_generate::ErrorTransform::empty()
    };

    let modules = doc.modules().map(|module| {
        let modname = names.module(&module.name);
        let fs = module
            .funcs()
            .map(|f| define_func(&names, &module, &f, &error_transform));
        let modtrait = define_module_trait(&names, &module, &error_transform);
        let ctx_type = names.ctx_type();
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
        &config.ctx_name,
        &config.constructor,
        &quote!(wasi_common::snapshots::preview_1),
        config.pre_hook.as_ref().unwrap_or(&empty),
        config.post_hook.as_ref().unwrap_or(&empty),
    ));

    TokenStream::from(ts)
}
