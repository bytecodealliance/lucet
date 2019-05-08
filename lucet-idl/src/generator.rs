use crate::error::IDLError;
use crate::module::Module;
use crate::pretty_writer::PrettyWriter;
use crate::types::{DataType, Ident, Named};
use std::io::Write;

pub trait Generator<W: Write> {
    fn gen_prelude(&mut self, pretty_writer: &mut PrettyWriter<W>) -> Result<(), IDLError>;

    fn gen_type_header(
        &mut self,
        module: &Module,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &Named<DataType>,
    ) -> Result<(), IDLError>;

    fn gen_alias(
        &mut self,
        module: &Module,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &Named<DataType>,
    ) -> Result<(), IDLError>;

    fn gen_struct(
        &mut self,
        module: &Module,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &Named<DataType>,
    ) -> Result<(), IDLError>;

    fn gen_enum(
        &mut self,
        module: &Module,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &Named<DataType>,
    ) -> Result<(), IDLError>;
    /*
        fn gen_function(
            &mut self,
            module: &Module,
            pretty_writer: &mut PrettyWriter<W>,
            func_decl: &Named<FuncDecl, '_>,
    */
    fn gen_for_id(
        &mut self,
        module: &Module,
        pretty_writer: &mut PrettyWriter<W>,
        id: Ident,
    ) -> Result<(), IDLError> {
        if let Some(data_type_entry) = module.get_datatype(id) {
            self.gen_type_header(module, pretty_writer, &data_type_entry)?;
            match &data_type_entry.entity {
                DataType::Struct { .. } => self.gen_struct(module, pretty_writer, &data_type_entry),
                DataType::Alias { .. } => self.gen_alias(module, pretty_writer, &data_type_entry),
                DataType::Enum { .. } => self.gen_enum(module, pretty_writer, &data_type_entry),
            }?;
        }
        Ok(())
    }
}
