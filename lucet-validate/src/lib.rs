use failure::Fail;
use witx;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Idk")]
    Idk,
}

pub fn validate(interface: &witx::Document, module_contents: &[u8]) -> Result<(), Error> {
    Err(Error::Idk)
}
