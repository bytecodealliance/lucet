use crate::names::Names;

use proc_macro2::TokenStream;
use quote::quote;
use witx::Layout;

pub(super) fn define_handle(
    names: &Names,
    name: &witx::Id,
    h: &witx::HandleDatatype,
) -> TokenStream {
    let rt = names.runtime_mod();
    let ident = names.type_(name);
    let size = h.mem_size_align().size as u32;
    let align = h.mem_size_align().align as usize;
    quote! {
        #[repr(transparent)]
        #[derive(Copy, Clone, Debug, ::std::hash::Hash, Eq, PartialEq)]
        pub struct #ident(u32);

        impl #ident {
            pub unsafe fn inner(&self) -> u32 {
                self.0
            }
        }

        impl From<#ident> for u32 {
            fn from(e: #ident) -> u32 {
                e.0
            }
        }

        impl From<#ident> for i32 {
            fn from(e: #ident) -> i32 {
                e.0 as i32
            }
        }

        impl From<u32> for #ident {
            fn from(e: u32) -> #ident {
                #ident(e)
            }
        }
        impl From<i32> for #ident {
            fn from(e: i32) -> #ident {
                #ident(e as u32)
            }
        }

        impl ::std::fmt::Display for #ident {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}({})", stringify!(#ident), self.0)
            }
        }

        impl<'a> #rt::GuestType<'a> for #ident {
            fn guest_size() -> u32 {
                #size
            }

            fn guest_align() -> usize {
                #align
            }

            fn read(location: &#rt::GuestPtr<'a, #ident>) -> Result<#ident, #rt::GuestError> {
                Ok(#ident(u32::read(&location.cast())?))
            }

            fn write(location: &#rt::GuestPtr<'_, Self>, val: Self) -> Result<(), #rt::GuestError> {
                u32::write(&location.cast(), val.0)
            }
        }

        unsafe impl<'a> #rt::GuestTypeTransparent<'a> for #ident {
            #[inline]
            fn validate(_location: *mut #ident) -> Result<(), #rt::GuestError> {
                // All bit patterns accepted
                Ok(())
            }
        }
    }
}

impl super::WiggleType for witx::HandleDatatype {
    fn impls_display(&self) -> bool {
        true
    }
}
