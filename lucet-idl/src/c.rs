use crate::pretty_writer::PrettyWriter;
use crate::{
    AbiType, AliasDatatype, AtomType, Datatype, DatatypeVariant, EnumDatatype, Function, IDLError,
    MemArea, Package, StructDatatype,
};
use std::io::Write;

/// Generator for the C backend
pub struct CGenerator {
    pub w: PrettyWriter,
}

impl CGenerator {
    pub fn new(w: Box<dyn Write>) -> Self {
        let mut w = PrettyWriter::new(w);
        let prelude = r"
#include <assert.h>
#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>";
        for line in prelude.lines() {
            w.write_line(line.as_ref());
        }
        w.eob();
        Self { w }
    }

    pub fn generate_guest(&mut self, package: &Package) -> Result<(), IDLError> {
        for module in package.modules() {
            if module.name() == "std" {
                continue;
            }
            for dt in module.datatypes() {
                self.gen_type_header(&dt)?;
                match dt.variant() {
                    DatatypeVariant::Struct(s) => self.gen_struct(&s)?,
                    DatatypeVariant::Alias(a) => self.gen_alias(&a)?,
                    DatatypeVariant::Enum(e) => self.gen_enum(&e)?,
                    DatatypeVariant::Atom(_) => unreachable!(),
                }
            }
            for func in module.functions() {
                self.gen_function(&func)?;
            }
        }
        Ok(())
    }

    fn gen_type_header(&mut self, dt: &Datatype) -> Result<(), IDLError> {
        self.w
            .eob()
            .writeln(format!("// ---------- {} ----------", dt.name()))
            .eob();
        Ok(())
    }

    // The most important thing in alias generation is to cache the size
    // and alignment rules of what it ultimately points to
    fn gen_alias(&mut self, alias: &AliasDatatype) -> Result<(), IDLError> {
        let own_type_name = Datatype::from(alias.clone()).c_type_name();
        self.w
            .writeln(format!(
                "typedef {} {};",
                alias.to().c_type_name(),
                own_type_name,
            ))
            .eob();

        // Add an assertion to check that resolved size is the one we computed
        self.w
            .writeln(format!(
                "_Static_assert(sizeof({}) == {}, \"unexpected alias size\");",
                own_type_name,
                alias.mem_size(),
            ))
            .eob();

        Ok(())
    }

    fn gen_struct(&mut self, struct_: &StructDatatype) -> Result<(), IDLError> {
        let own_type_name = Datatype::from(struct_.clone()).c_type_name();
        self.w.writeln(format!("{} {{", own_type_name));
        let mut w_block = self.w.new_block();
        for member in struct_.members() {
            w_block.writeln(format!(
                "{} {};",
                member.type_().c_type_name(),
                member.name(),
            ));
        }
        self.w.writeln("};").eob();

        // Skip the first member, as it will always be at the beginning of the structure
        for member in struct_.members().skip(1) {
            self.w.writeln(format!(
                "_Static_assert(offsetof({}, {}) == {}, \"unexpected offset\");",
                own_type_name,
                member.name(),
                member.offset()
            ));
        }

        self.w
            .writeln(format!(
                "_Static_assert(sizeof({}) == {}, \"unexpected structure size\");",
                own_type_name,
                struct_.mem_size(),
            ))
            .eob();
        Ok(())
    }

    // Enums generate both a specific typedef, and a traditional C-style enum
    // The typedef is required to use a native type which is consistent across all architectures
    fn gen_enum(&mut self, enum_: &EnumDatatype) -> Result<(), IDLError> {
        let own_type_name = Datatype::from(enum_.clone()).c_type_name();
        self.w.writeln(format!("{} {{", own_type_name));
        let mut w = self.w.new_block();
        for variant in enum_.variants() {
            w.writeln(format!(
                "{}, // {}",
                macro_for(enum_.name(), variant.name()),
                variant.index(),
            ));
        }
        self.w.writeln("};").eob();
        self.w
            .writeln(format!(
                "_Static_assert(sizeof({}) == {}, \"unexpected enumeration size\");",
                own_type_name,
                enum_.mem_size(),
            ))
            .eob();
        Ok(())
    }

    /// Currently support generating ABI level definition for C guests. Bindings not supported.
    fn gen_function(&mut self, func: &Function) -> Result<(), IDLError> {
        let rets = func.rets().collect::<Vec<_>>();
        let return_decl = match rets.len() {
            0 => "void".to_owned(),
            1 => rets[0].type_().c_type_name(),
            _ => unreachable!("functions limited to 0 or 1 return arguments"),
        };

        let arg_list = func
            .args()
            .map(|a| format!("{} {}", a.type_().c_type_name(), a.name()))
            .collect::<Vec<String>>()
            .join(", ");

        self.w.writeln(format!(
            "extern {} {}({});",
            return_decl,
            func.name(),
            arg_list,
        ));

        Ok(())
    }
}

trait CTypeName {
    fn c_type_name(&self) -> String;
}

impl CTypeName for AtomType {
    fn c_type_name(&self) -> String {
        match self {
            AtomType::Bool => "bool",
            AtomType::U8 => "uint8_t",
            AtomType::U16 => "uint16_t",
            AtomType::U32 => "uint32_t",
            AtomType::U64 => "uint64_t",
            AtomType::I8 => "int8_t",
            AtomType::I16 => "int16_t",
            AtomType::I32 => "int32_t",
            AtomType::I64 => "int64_t",
            AtomType::F32 => "float",
            AtomType::F64 => "double",
        }
        .to_owned()
    }
}

impl CTypeName for AbiType {
    fn c_type_name(&self) -> String {
        AtomType::from(self.clone()).c_type_name()
    }
}

impl CTypeName for Datatype<'_> {
    fn c_type_name(&self) -> String {
        match self.variant() {
            DatatypeVariant::Struct(_) => format!("struct {}", self.name()),
            DatatypeVariant::Enum(_) => format!("enum {}", self.name()),
            DatatypeVariant::Alias(_) => self.name().to_owned(),
            DatatypeVariant::Atom(a) => a.c_type_name(),
        }
    }
}

fn macro_for(prefix: &str, name: &str) -> String {
    use heck::ShoutySnakeCase;
    let mut macro_name = String::new();
    macro_name.push_str(&prefix.to_uppercase());
    macro_name.push('_');
    macro_name.push_str(&name.to_shouty_snake_case());
    macro_name
}
