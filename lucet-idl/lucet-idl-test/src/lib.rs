mod c_guest;
mod host;
mod rust_guest;
pub mod syntax;
mod test_plan;
mod values;
mod workspace;

pub use c_guest::CGuestApp;
pub use host::HostApp;
pub use rust_guest::RustGuestApp;
pub use syntax::Spec;
pub use test_plan::{FuncCallPredicate, ModuleTestPlan};
pub use values::DatatypeExt;
pub use workspace::Workspace;

#[cfg(test)]
mod tests {
    use crate::{CGuestApp, HostApp, ModuleTestPlan, RustGuestApp, Spec};
    use lucet_idl::parse_package;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn generate_idl(spec in Spec::strat(4)) {
            let rendered = spec.render_idl();
            println!("{}", rendered);
            let _ = parse_package(&rendered).unwrap();
        }

        #[test]
        fn generate_rust_guest(spec in Spec::strat(20)) {
            let rendered = spec.render_idl();
            let pkg = parse_package(&rendered).unwrap();
            let modules = pkg.modules().collect::<Vec<_>>();
            let test_plan = ModuleTestPlan::trivial(&modules.get(0).expect("just one module"));
            let mut rust_guest_app = RustGuestApp::new().expect("create rust guest app");
            let _rust_guest_so = rust_guest_app.build(&pkg, &test_plan).expect("compile rust guest app");
        }

        #[test]
        fn generate_c_guest(spec in Spec::strat(20)) {
            let rendered = spec.render_idl();
            let pkg = parse_package(&rendered).unwrap();
            let mut c_guest_app = CGuestApp::new().expect("create c guest app");
            let _c_guest_so = c_guest_app.build(&pkg).expect("compile c guest app");
        }

        #[test]
        fn generate_host(spec in Spec::strat(20)) {
            let rendered = spec.render_idl();
            let pkg = parse_package(&rendered).unwrap();
            let modules = pkg.modules().collect::<Vec<_>>();
            let test_plan = ModuleTestPlan::trivial(&modules.get(0).expect("just one module"));
            let mut host_app = HostApp::new(&pkg, &test_plan).expect("create host app");
            let _host_app = host_app.build().expect("compile host app");
        }

    }
}
