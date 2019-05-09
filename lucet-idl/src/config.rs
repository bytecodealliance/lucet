use super::backend::Backend;
use super::target::Target;
use crate::c::CGenerator;
use crate::generator::Generator;
use crate::rust::RustGenerator;
use std::io::Write;

#[derive(Default, Clone, Debug)]
pub struct Config {
    pub target: Target,
    pub backend: Backend,
}

impl Config {
    pub fn parse(target_opt: &str, backend_opt: &str, zero_native_pointers: bool) -> Self {
        let mut target = Target::from(target_opt);
        let backend = Backend::from(backend_opt);
        if zero_native_pointers {
            target = Target::Generic;
        }
        Self { target, backend }
    }

    pub fn generator(&self, w: Box<dyn Write>) -> Box<dyn Generator> {
        match self.backend {
            Backend::C => Box::new(CGenerator::new(self.target, w)),
            Backend::Rust => Box::new(RustGenerator::new(self.target, w)),
        }
    }
}
