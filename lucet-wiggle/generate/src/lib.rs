use heck::SnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub fn hostcall_name(m: &witx::Module, f: &witx::InterfaceFunc) -> String {
    format!(
        "hostcall_{}_{}",
        m.name.as_str().to_snake_case(),
        f.name.as_str().to_snake_case()
    )
}

pub fn generate(doc: &witx::Document, config: &wiggle_generate::Config) -> TokenStream {
    let names = wiggle_generate::Names::new(config);
    let fs = doc.modules().map(|m| {
        let fs = m.funcs().map(|f| {
            let name = format_ident!("{}", hostcall_name(&m, &f));
            let coretype = f.core_type();
            let func_args = coretype.args.iter().map(|a| {
                let name = names.func_core_arg(a);
                let atom = names.atom_type(a.repr());
                quote!(#name: #atom)
            });
            let call_args = coretype.args.iter().map(|a| {
                let name = names.func_core_arg(a);
                quote!(#name)
            });
            let rets = coretype
                .ret
                .as_ref()
                .map(|r| {
                    let atom = names.atom_type(r.repr());
                    quote!(#atom)
                })
                .unwrap_or(quote!(()));
            let mod_name = names.module(&m.name);
            let method_name = names.func(&f.name);
            let ctx_type = config.ctx.name.clone();
            quote! {
                #[lucet_hostcall]
                #[no_mangle]
                pub fn #name(vmctx: &mut lucet_runtime::vmctx::Vmctx, #(#func_args),*) -> #rets {
                    let mut memory: wiggle_runtime::GuestMemory = unimplemented!();
                    let mut ctx: #ctx_type = unimplemented!();
                    #mod_name::#method_name(&mut ctx, &mut memory, #(#call_args),*)
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
        use lucet_runtime::lucet_hostcall;
        #(#fs)*
        pub fn init() {
            let funcs: &[*const extern "C" fn()] = &[
                #(#init),*
            ];
            ::std::mem::forget(::std::rc::Rc::new(funcs));
        }
    }
}
