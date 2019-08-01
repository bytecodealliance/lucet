use crate::env::atoms::AtomType;
use crate::env::repr::{
    DatatypeIdent, DatatypeIx, DatatypeRepr, DatatypeVariantRepr, ModuleDatatypesRepr,
    ModuleFuncsRepr, ModuleIx, ModuleRepr,
};
use crate::env::MemArea;
use cranelift_entity::{EntityRef, PrimaryMap};

pub fn std_module() -> ModuleRepr {
    ModuleRepr {
        datatypes: std_datatypes(),
        funcs: ModuleFuncsRepr {
            names: PrimaryMap::new(),
            funcs: PrimaryMap::new(),
        },
    }
}

fn std_datatypes() -> ModuleDatatypesRepr {
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
    use super::std_module;
    use crate::env::atoms::AtomType;
    use crate::env::cursor::Package;
    use crate::env::repr::{DatatypeIdent, PackageRepr};
    use cranelift_entity::PrimaryMap;
    #[test]
    fn atom_idents() {
        use AtomType::*;

        let mut names = PrimaryMap::new();
        names.push("std".to_owned());
        let mut modules = PrimaryMap::new();
        modules.push(std_module());
        let pkg_repr = PackageRepr { names, modules };
        let prelude = Package::new(&pkg_repr);

        fn lookup_atom_by_id(package: &Package, ident: DatatypeIdent) -> AtomType {
            let dt = package.datatype_by_id(ident).expect("get by id");
            dt.variant().atom().expect("datatype is atom")
        }

        assert_eq!(Bool, lookup_atom_by_id(&prelude, Bool.datatype_id()));
        assert_eq!(U8, lookup_atom_by_id(&prelude, U8.datatype_id()));
        assert_eq!(U16, lookup_atom_by_id(&prelude, U16.datatype_id()));
        assert_eq!(U32, lookup_atom_by_id(&prelude, U32.datatype_id()));
        assert_eq!(U64, lookup_atom_by_id(&prelude, U64.datatype_id()));
        assert_eq!(I8, lookup_atom_by_id(&prelude, I8.datatype_id()));
        assert_eq!(I16, lookup_atom_by_id(&prelude, I16.datatype_id()));
        assert_eq!(I32, lookup_atom_by_id(&prelude, I32.datatype_id()));
        assert_eq!(I64, lookup_atom_by_id(&prelude, I64.datatype_id()));
        assert_eq!(F32, lookup_atom_by_id(&prelude, F32.datatype_id()));
        assert_eq!(F64, lookup_atom_by_id(&prelude, F64.datatype_id()));

        fn lookup_atom_by_name(package: &Package, name: &str) -> AtomType {
            let dt = package
                .module("std")
                .expect("std module exists")
                .datatype(name)
                .expect("get by name");
            dt.variant().atom().expect("datatype is atom")
        }

        assert_eq!(Bool, lookup_atom_by_name(&prelude, "bool"));
        assert_eq!(U8, lookup_atom_by_name(&prelude, "u8"));
        assert_eq!(U16, lookup_atom_by_name(&prelude, "u16"));
        assert_eq!(U32, lookup_atom_by_name(&prelude, "u32"));
        assert_eq!(U64, lookup_atom_by_name(&prelude, "u64"));
        assert_eq!(I8, lookup_atom_by_name(&prelude, "i8"));
        assert_eq!(I16, lookup_atom_by_name(&prelude, "i16"));
        assert_eq!(I32, lookup_atom_by_name(&prelude, "i32"));
        assert_eq!(I64, lookup_atom_by_name(&prelude, "i64"));
        assert_eq!(F32, lookup_atom_by_name(&prelude, "f32"));
        assert_eq!(F64, lookup_atom_by_name(&prelude, "f64"));
    }
}
