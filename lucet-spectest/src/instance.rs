use failure::{format_err, Error};
use lucet::{LucetError, UntypedRetval, Val};
use lucetc::program::Program;
pub use parity_wasm::elements::ValueType;
use parity_wasm::elements::{Internal, Type};

// some of the fields of this are not used, but they need to be stored
// because lifetimes
#[allow(dead_code)]
pub struct Instance {
    program: Program,
    lucet_module: lucet::Module,
    lucet_pool: lucet::Pool,
    lucet_instance: lucet::Instance,
}

impl Instance {
    pub fn new(
        program: Program,
        lucet_module: lucet::Module,
        lucet_pool: lucet::Pool,
        lucet_instance: lucet::Instance,
    ) -> Self {
        Self {
            program,
            lucet_module,
            lucet_pool,
            lucet_instance,
        }
    }

    pub fn run(&mut self, field: &str, args: &[Val]) -> Result<UntypedRetval, LucetError> {
        self.lucet_instance.run(field, args)
    }

    pub fn type_of(&self, field: &str) -> Result<ExportType, Error> {
        if let Some(ref export_section) = self.program.module().export_section() {
            export_section
                .entries()
                .iter()
                .find(|entry| entry.field() == field)
                .map(|entry| match entry.internal() {
                    Internal::Function(func_ix) => self.func_type(*func_ix),
                    Internal::Global(global_ix) => self.global_type(*global_ix),
                    _ => Err(format_err!(
                        "cannot take type of export \"{}\": {:?}",
                        field,
                        entry.internal()
                    ))?,
                })
                .ok_or_else(|| format_err!("cannot find export named \"{}\"", field))?
        } else {
            Err(format_err!("no exports to find \"{}\" in", field))
        }
    }

    fn func_type(&self, func_ix: u32) -> Result<ExportType, Error> {
        if let Some(func_section) = self.program.module().function_section() {
            if let Some(func_entry) = func_section.entries().get(func_ix as usize) {
                if let Some(type_section) = self.program.module().type_section() {
                    if let Some(Type::Function(func_type)) =
                        type_section.types().get(func_entry.type_ref() as usize)
                    {
                        Ok(ExportType::Function(
                            func_type.params().to_vec(),
                            func_type.return_type(),
                        ))
                    } else {
                        Err(format_err!(
                            "type ix {} out of bounds",
                            func_entry.type_ref()
                        ))
                    }
                } else {
                    Err(format_err!("no type section!"))
                }
            } else {
                Err(format_err!("func ix {} out of bounds", func_ix))
            }
        } else {
            Err(format_err!("no func section!"))
        }
    }

    fn global_type(&self, global_ix: u32) -> Result<ExportType, Error> {
        if let Some(global_section) = self.program.module().global_section() {
            if let Some(global_entry) = global_section.entries().get(global_ix as usize) {
                Ok(ExportType::Global(
                    global_entry.global_type().content_type(),
                ))
            } else {
                Err(format_err!("no such global {}", global_ix))
            }
        } else {
            Err(format_err!("no section to find global {}", global_ix))
        }
    }
}

#[derive(Debug)]
pub enum ExportType {
    Function(Vec<ValueType>, Option<ValueType>),
    Global(ValueType),
}
