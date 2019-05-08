use crate::error::IDLError;
use crate::module::Module;
use crate::types::{DataType, FuncDecl, Named};

pub trait Generator {
    fn gen_type_header(
        &mut self,
        module: &Module,
        data_type_entry: &Named<DataType>,
    ) -> Result<(), IDLError>;

    fn gen_alias(
        &mut self,
        module: &Module,
        data_type_entry: &Named<DataType>,
    ) -> Result<(), IDLError>;

    fn gen_struct(
        &mut self,
        module: &Module,
        data_type_entry: &Named<DataType>,
    ) -> Result<(), IDLError>;

    fn gen_enum(
        &mut self,
        module: &Module,
        data_type_entry: &Named<DataType>,
    ) -> Result<(), IDLError>;

    fn gen_function(
        &mut self,
        module: &Module,
        func_decl_entry: &Named<FuncDecl>,
    ) -> Result<(), IDLError>;

    fn gen_datatype(&mut self, module: &Module, dt: &Named<DataType>) -> Result<(), IDLError> {
        self.gen_type_header(module, dt)?;
        match &dt.entity {
            DataType::Struct { .. } => self.gen_struct(module, dt)?,
            DataType::Alias { .. } => self.gen_alias(module, dt)?,
            DataType::Enum { .. } => self.gen_enum(module, dt)?,
        }
        Ok(())
    }
}
