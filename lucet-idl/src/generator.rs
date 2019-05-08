use crate::error::IDLError;
use crate::module::Module;
use crate::types::{DataType, FuncDecl, Ident, Named};

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

    fn gen_for_id(&mut self, module: &Module, id: Ident) -> Result<(), IDLError> {
        if let Some(data_type_entry) = module.get_datatype(id) {
            self.gen_type_header(module, &data_type_entry)?;
            match &data_type_entry.entity {
                DataType::Struct { .. } => self.gen_struct(module, &data_type_entry),
                DataType::Alias { .. } => self.gen_alias(module, &data_type_entry),
                DataType::Enum { .. } => self.gen_enum(module, &data_type_entry),
            }?;
        } else {
            if let Some(func_decl_entry) = module.get_func_decl(id) {
                self.gen_function(module, &func_decl_entry)?;
            } else {
                unreachable!("identifier must be for a datatype or function declration")
            }
        }
        Ok(())
    }
}
