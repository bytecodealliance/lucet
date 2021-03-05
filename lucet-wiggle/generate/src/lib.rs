pub mod config;
pub use config::Config;

use heck::SnakeCase;
use lucet_module::bindings::Bindings;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};

pub fn hostcall_name(m: &witx::Module, f: &witx::InterfaceFunc) -> String {
    format!(
        "hostcall_{}_{}",
        m.name.as_str().to_snake_case(),
        f.name.as_str().to_snake_case()
    )
}
pub fn bindings(doc: &witx::Document) -> Bindings {
    let bs = doc
        .modules()
        .map(|m| {
            (
                m.name.as_str().to_owned(),
                m.funcs()
                    .map(|f| (f.name.as_str().to_owned(), hostcall_name(&m, &f)))
                    .collect(),
            )
        })
        .collect();
    Bindings::new(bs)
}

pub fn generate(
    doc: &witx::Document,
    ctx_constructor: &TokenStream,
    wiggle_mod_path: &TokenStream,
    pre_hook: &TokenStream,
    post_hook: &TokenStream,
    codegen_settings: &wiggle_generate::CodegenSettings,
) -> TokenStream {
    let names = wiggle_generate::Names::new(quote!(lucet_wiggle));
    let fs = doc.modules().map(|m| {
        let fs = m.funcs().map(|f| {
            let name = format_ident!("{}", hostcall_name(&m, &f));
            let (params, results) = f.wasm_signature();
            let arg_names = (0..params.len())
                .map(|i| Ident::new(&format!("arg{}", i), Span::call_site()))
                .collect::<Vec<_>>();
            let func_args = params.iter().enumerate().map(|(i, ty)| {
                let name = &arg_names[i];
                let atom = names.wasm_type(*ty);
                quote!(#name: #atom)
            });
            let ret_ty = match results.len() {
                0 => quote!(()),
                1 => names.wasm_type(results[0]),
                _ => panic!(
                    "lucet-wiggle only supports 0 or 1 result type. function {} has: {:?}",
                    hostcall_name(&m, &f),
                    results
                ),
            };
            let mod_name = names.module(&m.name);
            let method_name = names.func(&f.name);

            let body = if codegen_settings.is_async(&m, &f) {
                quote!(vmctx.block_on(async move {
                    super::#mod_name::#method_name(&ctx, &memory, #(#arg_names),*)
                }))
            } else {
                quote!(
                    super::#mod_name::#method_name(&ctx, &memory, #(#arg_names),*)
                )
            };
            quote! {
                #[lucet_hostcall]
                #[no_mangle]
                pub fn #name(vmctx: &lucet_runtime::vmctx::Vmctx, #(#func_args),*) -> #ret_ty {
                    { #pre_hook }
                    let memory = lucet_wiggle::runtime::LucetMemory::new(vmctx);
                    let mut ctx = #ctx_constructor;
                    let r = #body;
                    { #post_hook }
                    match r {
                        Ok(r) => { r },
                        Err(trap) => { lucet_runtime::lucet_hostcall_terminate!(trap); }
                    }
                }
            }
        });
        quote!(#(#fs)*)
    });

    let init = doc.modules().map(|m| {
        let fs = m.funcs().map(|f| {
            let name = format_ident!("{}", hostcall_name(&m, &f));
            quote!(#name as _)
        });
        quote!(#(#fs),*)
    });

    quote! {
        pub mod hostcalls {
            use lucet_runtime::lucet_hostcall;
            use #wiggle_mod_path::types::*;
            #(#fs)*
            /// Lucet-runtime expects hostcalls to be resolved by the runtime
            /// linker (dlopen). By calling `init` in your program, we ensure that
            /// each hostcall is reachable and not garbage-collected by the
            /// compile-time linker (ld).
            pub fn init() {
                let funcs: &[*const extern "C" fn()] = &[
                    #(#init),*
                ];
                for func in funcs {
                    assert_ne!(*func, std::ptr::null(), "hostcall address is not null");
                }
            }
        }
    }
}
