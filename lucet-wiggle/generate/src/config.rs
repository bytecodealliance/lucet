use proc_macro2::{Span, TokenStream};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Error, Ident, Result, Token,
};
use wiggle_generate::config as w;

#[derive(Debug, Clone)]
pub struct Config {
    pub wiggle: w::Config,
    pub constructor: TokenStream,
}

#[derive(Debug, Clone)]
pub enum ConfigField {
    Wiggle(w::ConfigField),
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
            _ => Ok(ConfigField::Wiggle(w::ConfigField::parse_pair(
                id.to_string().as_ref(),
                input,
                id.span(),
            )?)),
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
        for f in fields {
            match f {
                ConfigField::Constructor(c) => {
                    constructor = Some(c);
                }
                ConfigField::Wiggle { .. } => {} // Ignore
            }
        }
        Ok(Config {
            wiggle,
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
