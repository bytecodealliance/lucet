use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

/// This attribute generates a Lucet hostcall from a standalone Rust function that takes a `&mut
/// Vmctx` as its first argument.
///
/// It is important to use this attribute for hostcalls, rather than exporting them
/// directly. Otherwise the behavior of instance termination and timeouts are
/// undefined. Additionally, the attribute makes the resulting function `unsafe extern "C"`
/// regardless of how the function is defined, as this ABI is required for all hostcalls.
///
/// In most cases, you will want to also provide the `#[no_mangle]` attribute and `pub` visibility
/// in order for the hostcall to be exported from the final executable.
///
/// ```ignore
/// #[lucet_hostcall]
/// #[no_mangle]
/// pub fn yield_5(vmctx: &Vmctx) {
///     vmctx.yield_val(5);
/// }
/// ```
///
/// Note that `lucet-runtime` must be a dependency of any crate where this attribute is used, and it
/// may not be renamed (this restriction may be lifted once [this
/// issue](https://github.com/rust-lang/rust/issues/54363) is resolved).
#[proc_macro_attribute]
pub fn lucet_hostcall(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // determine whether we need to import from `lucet_runtime_internals`; this is useful if we want
    // to define a hostcall for a target (or tests, more concretely) that doesn't depend on
    // `lucet-runtime`
    let from_internals = {
        let pkg_name = std::env::var("CARGO_PKG_NAME").unwrap();
        match pkg_name.as_str() {
            "lucet-runtime-internals" | "lucet-runtime-tests" => true,
            _ => false,
        }
    };

    let mut hostcall = syn::parse_macro_input!(item as syn::ItemFn);
    if let Some(abi) = &hostcall.sig.abi {
        let err = quote_spanned! {abi.span()=>
            compile_error!("Functions annotated with `#[lucet_hostcall]` may not have an `extern` ABI");
        };
        return err.into();
    }
    let hostcall_ident = hostcall.sig.ident.clone();

    // use the same attributes and visibility as the impl hostcall
    let attrs = hostcall.attrs.clone();
    let vis = hostcall.vis.clone();

    // remove #[no_mangle] from the attributes of the impl hostcall if it's there
    hostcall
        .attrs
        .retain(|attr| !attr.path.is_ident("no_mangle"));
    // make the impl hostcall private
    hostcall.vis = syn::Visibility::Inherited;

    // modify the type signature of the exported raw hostcall based on the original signature
    let mut raw_sig = hostcall.sig.clone();

    // hostcalls are always unsafe
    raw_sig.unsafety = Some(syn::Token![unsafe](raw_sig.span()));

    // hostcalls are always extern "C"
    raw_sig.abi = Some(syn::parse_quote!(extern "C"));

    let vmctx_mod = if from_internals {
        quote! { lucet_runtime_internals::vmctx }
    } else {
        quote! { lucet_runtime::vmctx }
    };

    // replace the first argument to the raw hostcall with the vmctx pointer
    if let Some(arg0) = raw_sig.inputs.iter_mut().next() {
        let lucet_vmctx: syn::FnArg = syn::parse_quote!(vmctx_raw: *const #vmctx_mod::lucet_vmctx);
        *arg0 = lucet_vmctx;
    }

    // the args after the first to provide to the hostcall impl
    let impl_args = hostcall
        .sig
        .inputs
        .iter()
        .skip(1)
        .map(|arg| match arg {
            syn::FnArg::Receiver(_) => {
                // this case is an error, but we produce some valid rust code anyway so that the
                // compiler can produce a more meaningful error message at a later point
                let s = syn::Token![self](arg.span());
                quote!(#s)
            }
            syn::FnArg::Typed(syn::PatType { pat, .. }) => quote!(#pat),
        })
        .collect::<Vec<_>>();

    let termination_details = if from_internals {
        quote! { lucet_runtime_internals::instance::TerminationDetails }
    } else {
        quote! { lucet_runtime::TerminationDetails }
    };

    let res_ident = quote::format_ident!("res");

    let block_if_async = match raw_sig.asyncness.take() {
        Some(_) => {
            quote! { let #res_ident = vmctx.block_on(#res_ident); }
        }
        None => {
            quote! {}
        }
    };

    let raw_hostcall = quote! {
        #(#attrs)*
        #vis
        #raw_sig {
            #[inline(always)]
            #hostcall

            let vmctx = #vmctx_mod::Vmctx::from_raw(vmctx_raw);
            #vmctx_mod::VmctxInternal::instance_mut(&vmctx).uninterruptable(|| {
                let res = std::panic::catch_unwind(move || {
                    let vmctx = #vmctx_mod::Vmctx::from_raw(vmctx_raw);

                    let #res_ident = #hostcall_ident(&vmctx, #(#impl_args),*);

                    #block_if_async

                    #res_ident
                });
                match res {
                    Ok(res) => res,
                    Err(e) => {
                        let details = match e.downcast::<#termination_details>() {
                            Ok(details) => *details,
                            Err(e) => #termination_details::OtherPanic(e),
                        };
                        #vmctx_mod::Vmctx::from_raw(vmctx_raw).terminate_no_unwind(details)
                    }
                }
            })
        }
    };
    raw_hostcall.into()
}
