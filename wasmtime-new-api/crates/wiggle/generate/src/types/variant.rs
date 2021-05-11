use crate::lifetimes::LifetimeExt;
use crate::names::Names;

use proc_macro2::{Literal, TokenStream};
use quote::quote;
use witx::Layout;

pub(super) fn define_variant(names: &Names, name: &witx::Id, v: &witx::Variant) -> TokenStream {
    let rt = names.runtime_mod();
    let ident = names.type_(name);
    let size = v.mem_size_align().size as u32;
    let align = v.mem_size_align().align as usize;
    let contents_offset = v.payload_offset() as u32;

    let lifetime = quote!('a);
    let tag_ty = super::int_repr_tokens(v.tag_repr);

    let variants = v.cases.iter().map(|c| {
        let var_name = names.enum_variant(&c.name);
        if let Some(tref) = &c.tref {
            let var_type = names.type_ref(&tref, lifetime.clone());
            quote!(#var_name(#var_type))
        } else {
            quote!(#var_name)
        }
    });

    let read_variant = v.cases.iter().enumerate().map(|(i, c)| {
        let i = Literal::usize_unsuffixed(i);
        let variantname = names.enum_variant(&c.name);
        if let Some(tref) = &c.tref {
            let varianttype = names.type_ref(tref, lifetime.clone());
            quote! {
                #i => {
                    let variant_ptr = location.cast::<u8>().add(#contents_offset)?;
                    let variant_val = <#varianttype as #rt::GuestType>::read(&variant_ptr.cast())?;
                    Ok(#ident::#variantname(variant_val))
                }
            }
        } else {
            quote! { #i => Ok(#ident::#variantname), }
        }
    });

    let write_variant = v.cases.iter().enumerate().map(|(i, c)| {
        let variantname = names.enum_variant(&c.name);
        let write_tag = quote! {
            location.cast().write(#i as #tag_ty)?;
        };
        if let Some(tref) = &c.tref {
            let varianttype = names.type_ref(tref, lifetime.clone());
            quote! {
                #ident::#variantname(contents) => {
                    #write_tag
                    let variant_ptr = location.cast::<u8>().add(#contents_offset)?;
                    <#varianttype as #rt::GuestType>::write(&variant_ptr.cast(), contents)?;
                }
            }
        } else {
            quote! {
                #ident::#variantname => {
                    #write_tag
                }
            }
        }
    });

    let mut extra_derive = quote!();
    let enum_try_from = if v.cases.iter().all(|c| c.tref.is_none()) {
        let tryfrom_repr_cases = v.cases.iter().enumerate().map(|(i, c)| {
            let variant_name = names.enum_variant(&c.name);
            let n = Literal::usize_unsuffixed(i);
            quote!(#n => Ok(#ident::#variant_name))
        });
        let abi_ty = names.wasm_type(v.tag_repr.into());
        extra_derive = quote!(, Copy);
        quote! {
            impl TryFrom<#tag_ty> for #ident {
                type Error = #rt::GuestError;
                fn try_from(value: #tag_ty) -> Result<#ident, #rt::GuestError> {
                    match value {
                        #(#tryfrom_repr_cases),*,
                        _ => Err( #rt::GuestError::InvalidEnumValue(stringify!(#ident))),
                    }
                }
            }

            impl TryFrom<#abi_ty> for #ident {
                type Error = #rt::GuestError;
                fn try_from(value: #abi_ty) -> Result<#ident, #rt::GuestError> {
                    #ident::try_from(#tag_ty::try_from(value)?)
                }
            }
        }
    } else {
        quote!()
    };

    let (enum_lifetime, extra_derive) = if v.needs_lifetime() {
        (quote!(<'a>), quote!())
    } else {
        (quote!(), quote!(, PartialEq #extra_derive))
    };

    quote! {
        #[derive(Clone, Debug #extra_derive)]
        pub enum #ident #enum_lifetime {
            #(#variants),*
        }

        #enum_try_from

        impl<'a> #rt::GuestType<'a> for #ident #enum_lifetime {
            fn guest_size() -> u32 {
                #size
            }

            fn guest_align() -> usize {
                #align
            }

            fn read(location: &#rt::GuestPtr<'a, Self>)
                -> Result<Self, #rt::GuestError>
            {
                let tag = location.cast::<#tag_ty>().read()?;
                match tag {
                    #(#read_variant)*
                    _ => Err(#rt::GuestError::InvalidEnumValue(stringify!(#ident))),
                }

            }

            fn write(location: &#rt::GuestPtr<'_, Self>, val: Self)
                -> Result<(), #rt::GuestError>
            {
                match val {
                    #(#write_variant)*
                }
                Ok(())
            }
        }
    }
}

impl super::WiggleType for witx::Variant {
    fn impls_display(&self) -> bool {
        false
    }
}
