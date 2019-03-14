extern crate clap;
#[macro_use]
extern crate failure;
extern crate goblin;
#[cfg_attr(test, macro_use)]
extern crate lazy_static;
extern crate parity_wasm;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[cfg(test)]
extern crate siphasher;
#[macro_use]
extern crate xfailure;

mod errors;
mod functions_ids;
mod functions_names;
mod map;
mod patcher;
mod sections;
mod symbols;

#[cfg(test)]
mod tests;

pub use errors::*;
pub use patcher::*;
