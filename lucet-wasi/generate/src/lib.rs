extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use wiggle_generate::{define_func, define_module_trait, Names};

mod config;

#[proc_macro]
pub fn bindings(args: TokenStream) -> TokenStream {
    let config = parse_macro_input!(args as config::Config);
    let doc = wasi_common::wasi::metadata::document();
    let names = Names::new(&config.ctx_name);

    let modules = doc.modules().map(|module| {
        let modname = names.module(&module.name);
        let trait_name = names.trait_name(&module.name);
        let fs = module
            .funcs()
            .map(|f| define_func(&names, &f, quote!(#trait_name)));
        let modtrait = define_module_trait(&names, &module);
        let ctx_type = names.ctx_type();
        quote! {
            pub mod #modname {
                use super::#ctx_type;
                use wasi_common::wasi::types::*;
                #(#fs)*
                #modtrait
            }
        }
    });

    let mut ts = quote! {
        #(#modules)*
    };

    ts.extend(lucet_wiggle_generate::generate(
        &doc,
        &config.ctx_name,
        &config.constructor,
        &quote!(wasi_common::wasi),
    ));

    TokenStream::from(ts)
}
