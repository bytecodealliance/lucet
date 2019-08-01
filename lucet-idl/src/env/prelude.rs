use crate::env::atoms::AtomType;
use crate::env::repr::{
    DatatypeIdent, DatatypeIx, DatatypeRepr, DatatypeVariantRepr, ModuleDatatypesRepr,
    ModuleFuncsRepr, ModuleIx, ModuleRepr, PackageRepr,
};
use crate::env::MemArea;
use cranelift_entity::{EntityRef, PrimaryMap};

pub fn base_package() -> PackageRepr {
    let mut names = PrimaryMap::new();
    names.push("std".to_owned());
    let mut modules = PrimaryMap::new();
    modules.push(ModuleRepr {
        datatypes: atom_datatypes(),
        funcs: ModuleFuncsRepr {
            names: PrimaryMap::new(),
            funcs: PrimaryMap::new(),
        },
    });
    PackageRepr { names, modules }
}

fn atom_datatypes() -> ModuleDatatypesRepr {
    fn create_atom(repr: &mut ModuleDatatypesRepr, name: &str, atom: AtomType) {
        let ix = repr.names.push(name.to_owned());
        let mem_size = atom.mem_size();
        let mem_align = atom.mem_align();
        let dix = repr.datatypes.push(DatatypeRepr {
            variant: DatatypeVariantRepr::Atom(atom),
            mem_size,
            mem_align,
        });
        assert_eq!(ix, dix, "names and datatypes out of sync");
        repr.topological_order.push(ix);
    }

    let mut repr = ModuleDatatypesRepr {
        names: PrimaryMap::new(),
        datatypes: PrimaryMap::new(),
        topological_order: Vec::new(),
    };
    create_atom(&mut repr, "bool", AtomType::Bool);
    create_atom(&mut repr, "u8", AtomType::U8);
    create_atom(&mut repr, "u16", AtomType::U16);
    create_atom(&mut repr, "u32", AtomType::U32);
    create_atom(&mut repr, "u64", AtomType::U64);
    create_atom(&mut repr, "i8", AtomType::I8);
    create_atom(&mut repr, "i16", AtomType::I16);
    create_atom(&mut repr, "i32", AtomType::I32);
    create_atom(&mut repr, "i64", AtomType::I64);
    create_atom(&mut repr, "f32", AtomType::F32);
    create_atom(&mut repr, "f64", AtomType::F64);

    repr
}

impl AtomType {
    pub fn datatype_id(&self) -> DatatypeIdent {
        use AtomType::*;
        match self {
            Bool => DatatypeIdent::new(ModuleIx::new(0), DatatypeIx::new(0)),
            U8 => DatatypeIdent::new(ModuleIx::new(0), DatatypeIx::new(1)),
            U16 => DatatypeIdent::new(ModuleIx::new(0), DatatypeIx::new(2)),
            U32 => DatatypeIdent::new(ModuleIx::new(0), DatatypeIx::new(3)),
            U64 => DatatypeIdent::new(ModuleIx::new(0), DatatypeIx::new(4)),
            I8 => DatatypeIdent::new(ModuleIx::new(0), DatatypeIx::new(5)),
            I16 => DatatypeIdent::new(ModuleIx::new(0), DatatypeIx::new(6)),
            I32 => DatatypeIdent::new(ModuleIx::new(0), DatatypeIx::new(7)),
            I64 => DatatypeIdent::new(ModuleIx::new(0), DatatypeIx::new(8)),
            F32 => DatatypeIdent::new(ModuleIx::new(0), DatatypeIx::new(9)),
            F64 => DatatypeIdent::new(ModuleIx::new(0), DatatypeIx::new(10)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::base_package;
    use crate::env::atoms::AtomType;
    use crate::env::repr::{DatatypeIdent, DatatypeVariantRepr, PackageRepr};
    #[test]
    fn atom_idents() {
        use AtomType::*;
        let prelude = base_package();
        fn lookup_atom(package: &PackageRepr, ident: DatatypeIdent) -> AtomType {
            let module = package.modules.get(ident.module).expect("valid moduleix");
            let dt = module
                .datatypes
                .datatypes
                .get(ident.datatype)
                .expect("valid datatypeix");
            match dt.variant {
                DatatypeVariantRepr::Atom(a) => a,
                _ => panic!("expected atom datatype, got {:?}", dt),
            }
        }
        assert_eq!(Bool, lookup_atom(&prelude, Bool.datatype_id()));
        assert_eq!(U8, lookup_atom(&prelude, U8.datatype_id()));
        assert_eq!(U16, lookup_atom(&prelude, U16.datatype_id()));
        assert_eq!(U32, lookup_atom(&prelude, U32.datatype_id()));
        assert_eq!(U64, lookup_atom(&prelude, U64.datatype_id()));
        assert_eq!(I8, lookup_atom(&prelude, I8.datatype_id()));
        assert_eq!(I16, lookup_atom(&prelude, I16.datatype_id()));
        assert_eq!(I32, lookup_atom(&prelude, I32.datatype_id()));
        assert_eq!(I64, lookup_atom(&prelude, I64.datatype_id()));
        assert_eq!(F32, lookup_atom(&prelude, F32.datatype_id()));
        assert_eq!(F64, lookup_atom(&prelude, F64.datatype_id()));
    }
}
