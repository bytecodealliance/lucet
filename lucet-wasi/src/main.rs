#![deny(bare_trait_objects)]

#[macro_use]
extern crate clap;

use anyhow::{format_err, Error};
use clap::Arg;
use lucet_runtime::{self, DlModule, Limits, MmapRegion, Module, PublicKey, Region, RunResult};
use lucet_wasi::{self, WasiCtxBuilder, __wasi_exitcode_t};
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

struct Config<'a> {
    lucet_module: &'a str,
    guest_args: Vec<&'a str>,
    entrypoint: &'a str,
    preopen_dirs: Vec<(File, &'a str)>,
    limits: Limits,
    timeout: Option<Duration>,
    verify: bool,
    pk_path: Option<PathBuf>,
}

fn parse_humansized(desc: &str) -> Result<u64, Error> {
    use human_size::{Byte, ParsingError, Size, SpecificSize};
    match desc.parse::<Size>() {
        Ok(s) => {
            let bytes: SpecificSize<Byte> = s.into();
            Ok(bytes.value() as u64)
        }
        Err(ParsingError::MissingMultiple) => Ok(desc.parse::<u64>()?),
        Err(e) => Err(e)?,
    }
}

fn main() {
    // No-ops, but makes sure the linker doesn't throw away parts
    // of the runtime:
    lucet_runtime::lucet_internal_ensure_linked();
    lucet_wasi::export_wasi_funcs();

    let matches = app_from_crate!()
        .arg(
            Arg::with_name("entrypoint")
                .long("entrypoint")
                .takes_value(true)
                .default_value("_start")
                .help("Entrypoint to run within the WASI module"),
        )
        .arg(
            Arg::with_name("preopen_dirs")
                .required(false)
                .long("dir")
                .takes_value(true)
                .multiple(true)
                .number_of_values(1)
                .help("A directory to provide to the WASI guest")
                .long_help(
                    "Directories on the host can be provided to the WASI guest as part of a \
                     virtual filesystem. Each directory is specified as \
                     --dir `host_path:guest_path`, where `guest_path` specifies the path that will \
                     correspond to `host_path` for calls like `fopen` in the guest.\
                     \n\n\
                     For example, `--dir /home/host_user/wasi_sandbox:/sandbox` will make \
                     `/home/host_user/wasi_sandbox` available within the guest as `/sandbox`.\
                     \n\n\
                     Guests will be able to access any files and directories under the \
                     `host_path`, but will be unable to access other parts of the host \
                     filesystem through relative paths (e.g., `/sandbox/../some_other_file`) \
                     or through symlinks.",
                ),
        )
        .arg(
            Arg::with_name("lucet_module")
                .required(true)
                .help("Path to the `lucetc`-compiled WASI module"),
        )
        .arg(
            Arg::with_name("heap_memory_size")
                .long("max-heap-size")
                .takes_value(true)
                .default_value("4 GiB")
                .help("Maximum heap size (must be a multiple of 4 KiB)"),
        )
        .arg(
            Arg::with_name("heap_address_space_size")
                .long("heap-address-space")
                .takes_value(true)
                .default_value("8 GiB")
                .help("Maximum heap address space size (must be a multiple of 4 KiB, and >= `max-heap-size`)"),
        )
        .arg(
            Arg::with_name("stack_size")
                .long("stack-size")
                .takes_value(true)
                .default_value("8 MiB")
                .help("Maximum stack size (must be a multiple of 4 KiB)"),
        )
        .arg(
            Arg::with_name("timeout").long("timeout").takes_value(true).help("Number of milliseconds the instance will be allowed to run")
            )
        .arg(
            Arg::with_name("guest_args")
                .required(false)
                .multiple(true)
                .help("Arguments to the WASI `main` function"),
        )
        .arg(
            Arg::with_name("verify")
                .long("--signature-verify")
                .takes_value(false)
                .help("Verify the signature of the source file")
        )
        .arg(
            Arg::with_name("pk_path")
                .long("--signature-pk")
                .takes_value(true)
                .help("Path to the public key to verify the source code signature")
        )
        .get_matches();

    let entrypoint = matches.value_of("entrypoint").unwrap();

    let lucet_module = matches.value_of("lucet_module").unwrap();

    let preopen_dirs = matches
        .values_of("preopen_dirs")
        .map(|vals| {
            vals.map(|preopen_dir| {
                if let [host_path, guest_path] =
                    preopen_dir.split(':').collect::<Vec<&str>>().as_slice()
                {
                    let host_dir = File::open(host_path).unwrap();
                    (host_dir, *guest_path)
                } else {
                    println!("Invalid directory specification: {}", preopen_dir);
                    println!("{}", matches.usage());
                    std::process::exit(1);
                }
            })
            .collect()
        })
        .unwrap_or(vec![]);

    let heap_memory_size = matches
        .value_of("heap_memory_size")
        .ok_or_else(|| format_err!("missing heap memory size"))
        .and_then(|v| parse_humansized(v))
        .unwrap() as usize;

    let heap_address_space_size = matches
        .value_of("heap_address_space_size")
        .ok_or_else(|| format_err!("missing heap address space size"))
        .and_then(|v| parse_humansized(v))
        .unwrap() as usize;

    if heap_memory_size > heap_address_space_size {
        println!("`heap-address-space` must be at least as large as `max-heap-size`");
        println!("{}", matches.usage());
        std::process::exit(1);
    }

    let stack_size = matches
        .value_of("stack_size")
        .ok_or_else(|| format_err!("missing stack size"))
        .and_then(|v| parse_humansized(v))
        .unwrap() as usize;

    let timeout = matches
        .value_of("timeout")
        .map(|t| Duration::from_millis(t.parse::<u64>().unwrap()));

    let limits = Limits {
        heap_memory_size,
        heap_address_space_size,
        stack_size,
        globals_size: 0, // calculated from module
    };

    let guest_args = matches
        .values_of("guest_args")
        .map(|vals| vals.collect())
        .unwrap_or(vec![]);

    let verify = matches.is_present("verify");
    let pk_path = matches.value_of("pk_path").map(PathBuf::from);

    let config = Config {
        lucet_module,
        guest_args,
        entrypoint,
        preopen_dirs,
        limits,
        timeout,
        verify,
        pk_path,
    };

    run(config)
}

fn run(config: Config<'_>) {
    let exitcode = {
        // doing all of this in a block makes sure everything gets dropped before exiting
        let pk = match (config.verify, config.pk_path) {
            (false, _) => None,
            (true, Some(pk_path)) => {
                Some(PublicKey::from_file(pk_path).expect("public key can be loaded"))
            }
            (true, None) => panic!("signature verification requires a public key"),
        };
        let module = if let Some(pk) = pk {
            DlModule::load_and_verify(&config.lucet_module, pk)
                .expect("signed module can be loaded")
        } else {
            DlModule::load(&config.lucet_module).expect("module can be loaded")
        };
        let min_globals_size = module.initial_globals_size();
        let globals_size = ((min_globals_size + 4096 - 1) / 4096) * 4096;

        let region = MmapRegion::create(
            1,
            &Limits {
                globals_size,
                ..config.limits
            },
        )
        .expect("region can be created");

        // put the path to the module on the front for argv[0]
        let args = std::iter::once(config.lucet_module)
            .chain(config.guest_args.into_iter())
            .collect::<Vec<&str>>();
        let mut ctx = WasiCtxBuilder::new()
            .args(args.iter())
            .inherit_stdio()
            .inherit_env();
        for (dir, guest_path) in config.preopen_dirs {
            ctx = ctx.preopened_dir(dir, guest_path);
        }
        let mut inst = region
            .new_instance_builder(module as Arc<dyn Module>)
            .with_embed_ctx(ctx.build().expect("WASI ctx can be created"))
            .build()
            .expect("instance can be created");

        if let Some(timeout) = config.timeout {
            let kill_switch = inst.kill_switch();
            thread::spawn(move || {
                thread::sleep(timeout);
                // We may hit this line exactly when the guest exits, so sometimes `terminate` can
                // fail. That's still acceptable, so just ignore the result.
                kill_switch.terminate().ok();
            });
        }

        match inst.run(config.entrypoint, &[]) {
            // normal termination implies 0 exit code
            Ok(RunResult::Returned(_)) => 0,
            // none of the WASI hostcalls use yield yet, so this shouldn't happen
            Ok(RunResult::Yielded(_)) => panic!("lucet-wasi unexpectedly yielded"),
            Err(lucet_runtime::Error::RuntimeTerminated(
                lucet_runtime::TerminationDetails::Provided(any),
            )) => *any
                .downcast_ref::<__wasi_exitcode_t>()
                .expect("termination yields an exitcode"),
            Err(lucet_runtime::Error::RuntimeTerminated(
                lucet_runtime::TerminationDetails::Remote,
            )) => {
                println!("Terminated via remote kill switch (likely a timeout)");
                std::u32::MAX
            }
            Err(e) => panic!("lucet-wasi runtime error: {}", e),
        }
    };
    std::process::exit(exitcode as i32);
}
