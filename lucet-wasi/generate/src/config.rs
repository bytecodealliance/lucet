use proc_macro2::{Span, TokenStream};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Error, Ident, Result, Token,
};

#[derive(Debug, Clone)]
pub struct Config {
    pub ctx_name: Ident,
    pub constructor: TokenStream,
}

#[derive(Debug, Clone)]
pub enum ConfigField {
    CtxName(Ident),
    Constructor(TokenStream),
}

impl Parse for ConfigField {
    fn parse(input: ParseStream) -> Result<Self> {
        let id: Ident = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        match id.to_string().as_ref() {
            "constructor" => {
                let contents;
                let _lbrace = braced!(contents in input);
                Ok(ConfigField::Constructor(contents.parse()?))
            }
            "ctx" => Ok(ConfigField::CtxName(input.parse()?)),
            _ => Err(Error::new(id.span(), "expected `constructor` or `ctx`")),
        }
    }
}

impl Config {
    pub fn build(fields: impl Iterator<Item = ConfigField> + Clone, err_loc: Span) -> Result<Self> {
        let mut ctx_name = None;
        let mut constructor = None;
        for f in fields {
            match f {
                ConfigField::Constructor(c) => {
                    constructor = Some(c);
                }
                ConfigField::CtxName(n) => {
                    ctx_name = Some(n);
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
