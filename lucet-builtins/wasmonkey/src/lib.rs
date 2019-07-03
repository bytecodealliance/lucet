#[macro_use]
extern crate failure;

#[cfg_attr(test, macro_use)]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;

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

pub use crate::errors::*;
pub use crate::patcher::*;
