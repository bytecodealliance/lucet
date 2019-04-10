use lucet_runtime::{
    Error as RuntimeError, Module as LucetModule, Region as LucetRegion, UntypedRetVal, Val,
};
use std::sync::Arc;

// some of the fields of this are not used, but they need to be stored
// because lifetimes
#[allow(dead_code)]
pub struct Instance {
    lucet_module: Arc<dyn LucetModule>,
    lucet_region: Arc<dyn LucetRegion>,
    lucet_instance: lucet_runtime::InstanceHandle,
}

impl Instance {
    pub fn new(
        lucet_module: Arc<dyn LucetModule>,
        lucet_region: Arc<dyn LucetRegion>,
        lucet_instance: lucet_runtime::InstanceHandle,
    ) -> Self {
        Self {
            lucet_module,
            lucet_region,
            lucet_instance,
        }
    }

    pub fn run(&mut self, field: &str, args: &[Val]) -> Result<UntypedRetVal, RuntimeError> {
        let res = self.lucet_instance.run(field.as_bytes(), args);
        if let Err(_) = res {
            self.lucet_instance
                .reset()
                .expect("possible to reset instance");
        }
        res
    }
}
