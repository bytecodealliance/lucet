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
    syn::custom_keyword!(errors);
    syn::custom_keyword!(constructor);
    syn::custom_keyword!(pre_hook);
    syn::custom_keyword!(post_hook);
    syn::custom_keyword!(async_);
}

#[derive(Debug, Clone)]
pub struct Config {
    pub wiggle: w::Config,
    pub constructor: TokenStream,
    pub pre_hook: Option<TokenStream>,
    pub post_hook: Option<TokenStream>,
}

#[derive(Debug, Clone)]
pub enum ConfigField {
    Wiggle(w::ConfigField),
    Constructor(TokenStream),
    PreHook(TokenStream),
    PostHook(TokenStream),
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
            Ok(ConfigField::Wiggle(w::ConfigField::Witx(
                w::WitxConf::Paths(input.parse()?),
            )))
        } else if lookahead.peek(kw::witx_literal) {
            input.parse::<kw::witx_literal>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Wiggle(w::ConfigField::Witx(
                w::WitxConf::Literal(input.parse()?),
            )))
        } else if lookahead.peek(kw::async_) {
            input.parse::<kw::async_>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Wiggle(w::ConfigField::Async(input.parse()?)))
        } else if lookahead.peek(kw::errors) {
            input.parse::<kw::errors>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Wiggle(w::ConfigField::Error(input.parse()?)))
        } else {
            Err(lookahead.error())
        }
    }
}

impl Config {
    pub fn build(fields: impl Iterator<Item = ConfigField> + Clone, err_loc: Span) -> Result<Self> {
        let wiggle = w::Config::build(
            fields.clone().filter_map(|f| match f {
                ConfigField::Wiggle(w) => Some(w),
                _ => None,
            }),
            err_loc,
        )?;
        let mut constructor = None;
        let mut pre_hook = None;
        let mut post_hook = None;
        for f in fields {
            match f {
                ConfigField::Constructor(c) => {
                    constructor = Some(c);
                }
                ConfigField::PreHook(c) => {
                    pre_hook = Some(c);
                }
                ConfigField::PostHook(c) => {
                    post_hook = Some(c);
                }
                ConfigField::Wiggle { .. } => {} // Ignore
            }
        }
        Ok(Config {
            wiggle,
            constructor: constructor
                .take()
                .ok_or_else(|| Error::new(err_loc, "`constructor` field required"))?,
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
