use proc_macro2::{Span, TokenStream};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Error, Result, Token,
};
use wiggle_generate::config as w;

mod kw {
    syn::custom_keyword!(witx);
    syn::custom_keyword!(witx_literal);
    syn::custom_keyword!(ctx);
    syn::custom_keyword!(pre_hook);
    syn::custom_keyword!(post_hook);
    syn::custom_keyword!(target);
}

pub struct Config {
    pub witx: w::WitxConf,
    pub async_: w::AsyncFunctions,
    pub ctx: TokenStream,
    pub pre_hook: TokenStream,
    pub post_hook: TokenStream,
    pub target: syn::Path,
}

impl Config {
    pub fn is_async(&self, module: &str, field: &str) -> bool {
        match &self.async_ {
            w::AsyncFunctions::Some(fs) => fs
                .get(module)
                .and_then(|fs| fs.iter().find(|f| *f == field))
                .is_some(),
            w::AsyncFunctions::All => true,
        }
    }
}

pub enum ConfigField {
    Witx(w::WitxConf),
    Async(w::AsyncFunctions),
    Ctx(TokenStream),
    PreHook(TokenStream),
    PostHook(TokenStream),
    Target(syn::Path),
}

impl Parse for ConfigField {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::ctx) {
            input.parse::<kw::ctx>()?;
            input.parse::<Token![:]>()?;
            let contents;
            let _lbrace = braced!(contents in input);
            Ok(ConfigField::Ctx(contents.parse()?))
        } else if lookahead.peek(kw::pre_hook) {
            input.parse::<kw::pre_hook>()?;
            input.parse::<Token![:]>()?;
            let contents;
            let _lbrace = braced!(contents in input);
            Ok(ConfigField::PreHook(contents.parse()?))
        } else if lookahead.peek(kw::post_hook) {
            input.parse::<kw::post_hook>()?;
            input.parse::<Token![:]>()?;
            let contents;
            let _lbrace = braced!(contents in input);
            Ok(ConfigField::PostHook(contents.parse()?))
        } else if lookahead.peek(kw::witx) {
            input.parse::<kw::witx>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Witx(w::WitxConf::Paths(input.parse()?)))
        } else if lookahead.peek(kw::witx_literal) {
            input.parse::<kw::witx_literal>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Witx(w::WitxConf::Literal(input.parse()?)))
        } else if lookahead.peek(Token![async]) {
            input.parse::<Token![async]>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Async(input.parse()?))
        } else if lookahead.peek(kw::target) {
            input.parse::<kw::target>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Target(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

impl Config {
    pub fn build(fields: impl Iterator<Item = ConfigField>, err_loc: Span) -> Result<Self> {
        let mut witx = None;
        let mut async_ = None;
        let mut ctx = None;
        let mut pre_hook = None;
        let mut post_hook = None;
        let mut target = None;
        for f in fields {
            match f {
                ConfigField::Witx(c) => {
                    if witx.is_some() {
                        return Err(Error::new(err_loc, "duplicate `witx` field"));
                    }
                    witx = Some(c);
                }
                ConfigField::Async(c) => {
                    if async_.is_some() {
                        return Err(Error::new(err_loc, "duplicate `async` field"));
                    }
                    async_ = Some(c);
                }
                ConfigField::Ctx(c) => {
                    if ctx.is_some() {
                        return Err(Error::new(err_loc, "duplicate `ctx` field"));
                    }
                    ctx = Some(c);
                }
                ConfigField::PreHook(c) => {
                    if pre_hook.is_some() {
                        return Err(Error::new(err_loc, "duplicate `pre_hook` field"));
                    }
                    pre_hook = Some(c);
                }
                ConfigField::PostHook(c) => {
                    if post_hook.is_some() {
                        return Err(Error::new(err_loc, "duplicate `post_hook` field"));
                    }
                    post_hook = Some(c);
                }
                ConfigField::Target(c) => {
                    if target.is_some() {
                        return Err(Error::new(err_loc, "duplicate `target` field"));
                    }
                    target = Some(c);
                }
            }
        }
        Ok(Config {
            witx: witx
                .take()
                .ok_or_else(|| Error::new(err_loc, "`witx` field required"))?,
            async_: async_.take().unwrap_or_else(w::AsyncFunctions::default),
            ctx: ctx
                .take()
                .ok_or_else(|| Error::new(err_loc, "`ctx` field required"))?,
            pre_hook: pre_hook.take().unwrap_or_else(TokenStream::new),
            post_hook: post_hook.take().unwrap_or_else(TokenStream::new),
            target: target
                .take()
                .ok_or_else(|| Error::new(err_loc, "`target` field required"))?,
        })
    }
}

impl Parse for Config {
    fn parse(input: ParseStream) -> Result<Self> {
        let contents;
        let _lbrace = braced!(contents in input);
        let fields: Punctuated<ConfigField, Token![,]> =
            contents.parse_terminated(ConfigField::parse)?;
        Ok(Config::build(fields.into_iter(), input.span())?)
    }
}
