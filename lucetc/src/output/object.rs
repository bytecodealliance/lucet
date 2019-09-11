#![allow(dead_code)]
#![allow(unused_variables)]

use crate::name::Name;
use lucet_module::FunctionSpec;

use cranelift_object::ObjectProduct;
use failure::{format_err, Error};
use object::write::Object;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct ObjectFile {
    obj: Object,
}
impl ObjectFile {
    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let _ = path.as_ref().file_name().ok_or(format_err!(
            "path {:?} needs to have filename",
            path.as_ref()
        ));

        let mut file = File::create(path)?;
        let bytes = self.obj.write().map_err(failure::err_msg)?;
        file.write_all(&bytes)?;
        Ok(())
    }

    pub fn new(
        product: ObjectProduct,
        module_data_len: usize,
        function_manifest: Vec<(String, FunctionSpec)>,
        table_manifest: Vec<Name>,
    ) -> Result<Self, Error> {
        unimplemented!()
    }
}
