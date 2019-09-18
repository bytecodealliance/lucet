use crate::values::*;
use heck::SnakeCase;
use lucet_idl::{
    pretty_writer::PrettyWriter, BindingDirection, Function, Module, RustFunc, RustName,
    RustTypeName,
};
use proptest::prelude::*;

#[derive(Debug, Clone)]
pub struct FuncCallPredicate {
    func_name: String,
    func_call_args: Vec<String>,
    func_call_rets: Vec<String>,
    func_sig_args: Vec<String>,
    func_sig_rets: Vec<String>,
    pre: Vec<BindingVal>,
    post: Vec<BindingVal>,
}

impl FuncCallPredicate {
    pub fn strat(func: &Function) -> BoxedStrategy<FuncCallPredicate> {
        let args = func.rust_idiom_args();
        let rets = func.rust_idiom_rets();

        // Precondition on all arguments
        let pre_strat: Vec<BoxedStrategy<BindingVal>> =
            args.iter().map(|a| BindingVal::arg_strat(a)).collect();

        // Postcondition on all inout arguments, and all return values
        let post_strat: Vec<BoxedStrategy<BindingVal>> = args
            .iter()
            .filter(|a| a.direction() == BindingDirection::InOut)
            .map(|a| BindingVal::arg_strat(a))
            .chain(rets.iter().map(|r| BindingVal::ret_strat(r)))
            .collect();

        let func_call_args = args.iter().map(|a| a.arg_value()).collect::<Vec<_>>();
        let func_sig_args = args.iter().map(|a| a.arg_declaration()).collect::<Vec<_>>();

        let func_call_rets = rets.iter().map(|r| r.name()).collect::<Vec<_>>();
        let func_sig_rets = rets.iter().map(|a| a.ret_declaration()).collect::<Vec<_>>();

        let func_name = func.rust_name();
        (pre_strat, post_strat)
            .prop_map(move |(pre, post)| FuncCallPredicate {
                func_name: func_name.clone(),
                func_call_args: func_call_args.clone(),
                func_call_rets: func_call_rets.clone(),
                func_sig_args: func_sig_args.clone(),
                func_sig_rets: func_sig_rets.clone(),
                pre,
                post,
            })
            .boxed()
    }

    pub fn trivial(func: &Function) -> FuncCallPredicate {
        let args = func.rust_idiom_args();
        let rets = func.rust_idiom_rets();

        let pre = args.iter().map(|a| BindingVal::arg_trivial(a)).collect();
        let post = args
            .iter()
            .filter(|a| a.direction() == BindingDirection::InOut)
            .map(|a| BindingVal::arg_trivial(a))
            .chain(rets.iter().map(|r| BindingVal::ret_trivial(r)))
            .collect();

        let func_call_args = args.iter().map(|a| a.arg_value()).collect::<Vec<_>>();
        let func_sig_args = args.iter().map(|a| a.arg_declaration()).collect::<Vec<_>>();

        let func_call_rets = rets.iter().map(|r| r.name()).collect::<Vec<_>>();
        let func_sig_rets = rets.iter().map(|a| a.ret_declaration()).collect::<Vec<_>>();

        FuncCallPredicate {
            func_name: func.rust_name(),
            func_call_args,
            func_sig_args,
            func_call_rets,
            func_sig_rets,
            pre,
            post,
        }
    }

    pub fn render_caller(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .pre
            .iter()
            .map(|val| val.render_rust_binding())
            .collect();

        lines.push(format!(
            "let {} = {}({}).unwrap();",
            render_tuple(&self.func_call_rets, "_"),
            self.func_name,
            self.func_call_args.join(",")
        ));
        lines.append(
            &mut self
                .post
                .iter()
                .map(|val| {
                    format!(
                        "assert_eq!({}, {});",
                        val.name,
                        val.render_rust_constructor()
                    )
                })
                .collect(),
        );
        lines
    }

    pub fn render_callee(&self, w: &mut PrettyWriter) {
        w.writeln(format!(
            "fn {}(&mut self, {}) -> Result<{}, ()> {{",
            self.func_name,
            self.func_sig_args.join(", "),
            render_tuple(&self.func_sig_rets, "()")
        ))
        .indent();
        // Assert preconditions hold
        w.writelns(
            &self
                .pre
                .iter()
                .map(|val| format!("assert_eq!({}, {});", val.name, val.render_rust_ref()))
                .collect::<Vec<_>>(),
        );
        // Make postconditions hold
        let mut ret_vals = Vec::new();
        for post in self.post.iter() {
            match post.variant {
                BindingValVariant::Value(ref val) => {
                    ret_vals.push(val.render_rustval());
                }
                BindingValVariant::Ptr(ref val) => {
                    w.writeln(format!("*{} = {};", post.name, val.render_rustval()));
                }
                BindingValVariant::Array(ref vals) => {
                    for (ix, val) in vals.iter().enumerate() {
                        w.writeln(format!("{}[{}] = {};", post.name, ix, val.render_rustval()));
                    }
                }
            }
        }
        w.writeln(format!("Ok({})", render_tuple(&ret_vals, "()")));
        w.eob().writeln("}");
    }
}

#[derive(Debug, Clone)]
pub struct ModuleTestPlan {
    pub module_name: String,
    module_type_name: String,
    func_predicates: Vec<FuncCallPredicate>,
}

impl ModuleTestPlan {
    pub fn trivial(module: &Module) -> Self {
        let module_name = module.name().to_snake_case();
        let module_type_name = module.rust_type_name();
        let func_predicates = module
            .functions()
            .map(|f| FuncCallPredicate::trivial(&f))
            .collect();
        ModuleTestPlan {
            module_name,
            module_type_name,
            func_predicates,
        }
    }

    pub fn strat(module: &Module) -> BoxedStrategy<ModuleTestPlan> {
        let module_name = module.name().to_snake_case();
        let module_type_name = module.rust_type_name();
        module
            .functions()
            .map(|f| FuncCallPredicate::strat(&f))
            .collect::<Vec<_>>()
            .prop_map(move |func_predicates| ModuleTestPlan {
                module_name: module_name.clone(),
                module_type_name: module_type_name.clone(),
                func_predicates,
            })
            .boxed()
    }

    pub fn render_guest(&self, w: &mut PrettyWriter) {
        for func in self.func_predicates.iter() {
            w.writelns(&func.render_caller());
        }
    }

    pub fn render_host(&self, mut w: &mut PrettyWriter) {
        w.writeln(format!("use crate::idl::{}::*;", self.module_name));
        w.writeln("pub struct TestHarness;");
        w.writeln(format!("impl {} for TestHarness {{", self.module_type_name,))
            .indent();
        for func in self.func_predicates.iter() {
            func.render_callee(&mut w)
        }
        w.eob().writeln("}");
        w.writeln(format!(
            "pub fn ctx() -> Box<dyn {}> {{ Box::new(TestHarness) }}",
            self.module_type_name
        ));
    }
}
