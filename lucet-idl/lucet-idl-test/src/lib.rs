pub mod compile;
pub mod syntax;

#[cfg(test)]
mod tests {
    use crate::compile;
    use crate::syntax::Spec;
    use lucet_idl::parse_package;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn generate_and_rust(spec in Spec::strat(20)) {
            let rendered = spec.render_idl();
            let pkg = parse_package(&rendered).unwrap();
            compile::rust_codegen(&pkg);
        }

        #[test]
        fn generate_and_c(spec in Spec::strat(20)) {
            let rendered = spec.render_idl();
            let pkg = parse_package(&rendered).unwrap();
            compile::c_codegen(&pkg);
        }
    }
}
