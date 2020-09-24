use proc_macro2::{Span, TokenStream};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Error, Ident, Result, Token,
};
use wiggle_generate::config::ErrorConf;

#[derive(Debug, Clone)]
pub struct Config {
    pub ctx_name: Ident,
    pub constructor: TokenStream,
    pub error_conf: Option<ErrorConf>,
    pub pre_hook: Option<TokenStream>,
    pub post_hook: Option<TokenStream>,
}

#[derive(Debug, Clone)]
pub enum ConfigField {
    CtxName(Ident),
    Constructor(TokenStream),
    PreHook(TokenStream),
    PostHook(TokenStream),
    Error(ErrorConf),
}
mod kw {
    syn::custom_keyword!(ctx);
    syn::custom_keyword!(constructor);
    syn::custom_keyword!(pre_hook);
    syn::custom_keyword!(post_hook);
    syn::custom_keyword!(errors);
}

impl Parse for ConfigField {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::constructor) {
            input.parse::<kw::constructor>()?;
            input.parse::<Token![:]>()?;
            let contents;
            let _lbrace = braced!(contents in input);
            Ok(ConfigField::Constructor(contents.parse()?))
        } else if lookahead.peek(kw::ctx) {
            input.parse::<kw::ctx>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::CtxName(input.parse()?))
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
        } else if lookahead.peek(kw::errors) {
            input.parse::<kw::errors>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Error(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

impl Config {
    pub fn build(fields: impl Iterator<Item = ConfigField> + Clone, err_loc: Span) -> Result<Self> {
        let mut ctx_name = None;
        let mut constructor = None;
        let mut pre_hook = None;
        let mut post_hook = None;
        let mut error_conf = None;
        for f in fields {
            match f {
                ConfigField::Constructor(c) => {
                    constructor = Some(c);
                }
                ConfigField::CtxName(n) => {
                    ctx_name = Some(n);
                }
                ConfigField::PreHook(c) => {
                    pre_hook = Some(c);
                }
                ConfigField::PostHook(c) => {
                    post_hook = Some(c);
                }
                ConfigField::Error(c) => {
                    error_conf = Some(c);
                }
            }
        }
        Ok(Config {
            ctx_name: ctx_name
                .take()
                .ok_or_else(|| Error::new(err_loc, "`ctx` field required"))?,
            constructor: constructor
                .take()
                .ok_or_else(|| Error::new(err_loc, "`constructor` field required"))?,
            error_conf: error_conf.take(),
            pre_hook,
            post_hook,
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
