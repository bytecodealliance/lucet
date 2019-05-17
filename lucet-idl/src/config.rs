use crate::backend::Backend;
use crate::c::CGenerator;
use crate::error::IDLError;
use crate::generator::Generator;
use crate::rust::RustGenerator;
use std::io::Write;

#[derive(Clone, Debug)]
pub struct Config {
    pub backend: Backend,
}

impl Config {
    pub fn parse(backend_opt: &str) -> Result<Self, IDLError> {
        let backend = Backend::from_str(backend_opt)
            .ok_or_else(|| IDLError::UsageError(format!("Invalid backend: {}", backend_opt)))?;
        Ok(Self { backend })
    }

    pub fn generator(&self, w: Box<dyn Write>) -> Box<dyn Generator> {
        match self.backend {
            Backend::CGuest => Box::new(CGenerator::new(w)),
            Backend::RustGuest => Box::new(RustGenerator::new(w)),
            Backend::RustHost => unimplemented!(),
        }
    }
}
