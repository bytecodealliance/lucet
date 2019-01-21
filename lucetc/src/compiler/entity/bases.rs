use super::POINTER_SIZE;
use crate::compiler::Compiler;
use cranelift_codegen::ir::{self, types::I64, GlobalValueData};

// VMContext points directly to the heap (offset 0).
// Directly before the heap is a pointer to the globals (offset -POINTER_SIZE).
const GLOBAL_BASE_OFFSET: i32 = -1 * POINTER_SIZE as i32;

pub struct GlobalBases {
    heap: Option<ir::GlobalValue>,
    globals: Option<ir::GlobalValue>,
    table: Option<ir::GlobalValue>,
}

impl GlobalBases {
    pub fn new() -> Self {
        Self {
            heap: None,
            globals: None,
            table: None,
        }
    }

    pub fn table(&mut self, func: &mut ir::Function, compiler: &Compiler) -> ir::GlobalValue {
        self.table.unwrap_or_else(|| {
            let table = &compiler.prog.tables()[0];
            let gv = func.create_global_value(GlobalValueData::Symbol {
                name: compiler
                    .get_table(table)
                    .expect("table must be declared")
                    .into(),
                offset: 0.into(), // Symbol points directly to table.
                colocated: true,
            });
            self.table = Some(gv);
            gv
        })
    }

    pub fn heap(&mut self, func: &mut ir::Function, _compiler: &Compiler) -> ir::GlobalValue {
        self.heap.unwrap_or_else(|| {
            let gv = func.create_global_value(GlobalValueData::VMContext);
            self.heap = Some(gv);
            gv
        })
    }

    pub fn globals(&mut self, func: &mut ir::Function, _compiler: &Compiler) -> ir::GlobalValue {
        self.globals.unwrap_or_else(|| {
            let vmctx = func.create_global_value(GlobalValueData::VMContext);
            let gv = func.create_global_value(GlobalValueData::Load {
                base: vmctx,
                offset: GLOBAL_BASE_OFFSET.into(),
                global_type: I64,
                readonly: false,
            });
            self.globals = Some(gv);
            gv
        })
    }
}
