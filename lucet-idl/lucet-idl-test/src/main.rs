use clap::{App, Arg};
use env_logger;
use log::{debug, info};
use lucet_idl::{parse_package, Package};
use lucet_idl_test::{CGuestApp, HostApp, RustGuestApp, Spec, Workspace};
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::process;

fn main() {
    env_logger::init();
    let exe_config = ExeConfig::parse();

    let input_idl = match exe_config.input {
        Some(path) => read_to_string(path).expect("read contents of input file"),
        None => {
            let mut runner = TestRunner::default();
            let spec = Spec::strat(10).new_tree(&mut runner).unwrap().current();
            let rendered = spec.render_idl();
            info!("generated spec:\n{}", rendered);
            rendered
        }
    };

    let pkg = parse_package(&input_idl).expect("parse generated package");

    debug!("parsed package: {:?}", pkg);

    if exe_config.generate_values {
        generate_values(&pkg);
        process::exit(0);
    }

    // Workspace deleted when dropped - need to keep it alive for app to be run
    let mut guest_apps: Vec<(PathBuf, Workspace)> = Vec::new();

    if exe_config.build_rust_guest {
        let mut rust_guest_app = RustGuestApp::new().expect("create rust guest app");
        let rust_guest_so = rust_guest_app.build(&pkg).expect("compile rust guest app");
        guest_apps.push((rust_guest_so, rust_guest_app.into_workspace()));
    }

    if exe_config.build_c_guest {
        let mut c_guest_app = CGuestApp::new().expect("create c guest app");
        let c_guest_so = c_guest_app.build(&pkg).expect("compile c guest app");
        guest_apps.push((c_guest_so, c_guest_app.into_workspace()));
    }

    if exe_config.build_host {
        let mut host_app = HostApp::new(&pkg).expect("create host app");
        if exe_config.run_guests {
            for (guest_app_path, _ws) in guest_apps.iter() {
                host_app.run(guest_app_path).expect("run guest app");
            }
        }
    }
}

#[derive(Clone, Debug)]
struct ExeConfig {
    pub input: Option<PathBuf>,
    pub build_host: bool,
    pub build_rust_guest: bool,
    pub build_c_guest: bool,
    pub run_guests: bool,
    pub generate_values: bool,
}

impl ExeConfig {
    pub fn parse() -> Self {
        let matches = App::new("lucet-idl-test")
            .version("0.1.0")
            .about("lucet-idl testing tool")
            .arg(
                Arg::with_name("input")
                    .required(false)
                    .help("Path to the input idl file. If not provided, input will be generated"),
            )
            .arg(
                Arg::with_name("no_host")
                    .required(false)
                    .takes_value(false)
                    .long("no-host")
                    .help(""),
            )
            .arg(
                Arg::with_name("no_c_guest")
                    .required(false)
                    .takes_value(false)
                    .long("no-c-guest")
                    .help(""),
            )
            .arg(
                Arg::with_name("no_rust_guest")
                    .required(false)
                    .takes_value(false)
                    .long("no-rust-guest")
                    .help(""),
            )
            .arg(
                Arg::with_name("no_run")
                    .required(false)
                    .takes_value(false)
                    .long("no-run")
                    .help(""),
            )
            .arg(
                Arg::with_name("generate_values")
                    .required(false)
                    .takes_value(false)
                    .long("generate-values")
                    .help(""),
            )
            .get_matches();

        ExeConfig {
            input: matches.value_of("input").map(PathBuf::from),
            build_host: !matches.is_present("no_host"),
            build_c_guest: !matches.is_present("no_c_guest"),
            build_rust_guest: !matches.is_present("no_rust_guest"),
            run_guests: !matches.is_present("no_run") || !matches.is_present("no_host"),
            generate_values: matches.is_present("generate_values"),
        }
    }
}

fn generate_values(package: &Package) {
    use lucet_idl_test::values::*;

    for (_, m) in package.modules.iter() {
        for dt in m.datatypes() {
            let dt_generator = m.datatype_strat(&dt.datatype_ref());
            let mut runner = TestRunner::default();
            let value = dt_generator
                .new_tree(&mut runner)
                .expect("create valuetree")
                .current();
            println!("type: {:?}\nvalue: {:?}", dt, value);
        }
    }
}
