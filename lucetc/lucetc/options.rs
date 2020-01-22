use anyhow::Error;
use clap::{Arg, ArgMatches, Values};
use lucetc::{CpuFeatures, HeapSettings, OptLevel, SpecificFeature, TargetCpu};
use std::path::PathBuf;
use std::str::FromStr;
use target_lexicon::{Architecture, Triple};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenOutput {
    Clif,
    Obj,
    SharedObj,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorStyle {
    Human,
    Json,
}

impl Default for ErrorStyle {
    fn default() -> Self {
        ErrorStyle::Human
    }
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

fn humansized(bytes: u64) -> String {
    use human_size::{Byte, Mebibyte, SpecificSize};
    let bytes = SpecificSize::new(bytes as f64, Byte).expect("bytes");
    let mb: SpecificSize<Mebibyte> = bytes.into();
    mb.to_string()
}

fn cpu_features_from_args(cpu: Option<&str>, features: Values) -> Result<CpuFeatures, Error> {
    use SpecificFeature::*;
    use TargetCpu::*;
    if cpu.is_none() && features.len() == 0 {
        Ok(CpuFeatures::detect_cpuid())
    } else {
        let cpu: TargetCpu = match cpu {
            None => Baseline,
            Some(s) => match s.to_lowercase().as_str() {
                "native" => Native,
                "baseline" => Baseline,
                "nehalem" => Nehalem,
                "sandybridge" => Sandybridge,
                "haswell" => Haswell,
                "broadwell" => Broadwell,
                "skylake" => Skylake,
                "cannonlake" => Cannonlake,
                "icelake" => Icelake,
                "znver1" => Znver1,
                _ => unreachable!("invalid CPU string despite passing validation: {}", s),
            },
        };
        let specific_features = features
            .map(|fstr| {
                let b = match fstr.chars().nth(0) {
                    Some('+') => true,
                    Some('-') => false,
                    _ => unreachable!(
                        "invalid feature string despite passing validation: {}",
                        fstr
                    ),
                };
                // the only valid starting characters are single-byte '+' and '-', so this indexing
                // ought not to fail
                let f = match &fstr[1..] {
                    "sse3" => SSE3,
                    "ssse3" => SSSE3,
                    "sse41" => SSE41,
                    "sse42" => SSE42,
                    "popcnt" => Popcnt,
                    "avx" => AVX,
                    "bmi1" => BMI1,
                    "bmi2" => BMI2,
                    "lzcnt" => Lzcnt,
                    _ => unreachable!(
                        "invalid feature string despite passing validation: {}",
                        fstr
                    ),
                };
                (f, b)
            })
            .collect();
        Ok(CpuFeatures::new(cpu, specific_features))
    }
}

#[derive(Debug)]
pub struct Options {
    pub output: PathBuf,
    pub input: Vec<PathBuf>,
    pub codegen: CodegenOutput,
    pub binding_files: Vec<PathBuf>,
    pub witx_specs: Vec<PathBuf>,
    pub wasi_exe: bool,
    pub builtins_path: Option<PathBuf>,
    pub min_reserved_size: Option<u64>,
    pub max_reserved_size: Option<u64>,
    pub reserved_size: Option<u64>,
    pub guard_size: Option<u64>,
    pub opt_level: OptLevel,
    pub cpu_features: CpuFeatures,
    pub keygen: bool,
    pub sign: bool,
    pub verify: bool,
    pub pk_path: Option<PathBuf>,
    pub sk_path: Option<PathBuf>,
    pub count_instructions: bool,
    pub error_style: ErrorStyle,
    pub target: Triple,
}

impl Options {
    pub fn from_args(m: &ArgMatches<'_>) -> Result<Self, Error> {
        let input: Vec<PathBuf> = m
            .values_of("input")
            .unwrap_or_default()
            .map(PathBuf::from)
            .collect();

        let output = PathBuf::from(m.value_of("output").unwrap_or("a.out"));

        let binding_files: Vec<PathBuf> = m
            .values_of("bindings")
            .unwrap_or_default()
            .map(PathBuf::from)
            .collect();

        let witx_specs: Vec<PathBuf> = m
            .values_of("witx_specs")
            .unwrap_or_default()
            .map(PathBuf::from)
            .collect();
        let wasi_exe = m.is_present("wasi_exe");

        let codegen = match m.value_of("emit") {
            None => CodegenOutput::SharedObj,
            Some("clif") => CodegenOutput::Clif,
            Some("obj") => CodegenOutput::Obj,
            Some("so") => CodegenOutput::SharedObj,
            Some(_) => panic!("unknown value for emit"),
        };

        let builtins_path = m.value_of("builtins").map(PathBuf::from);

        let min_reserved_size = if let Some(min_reserved_str) = m.value_of("min_reserved_size") {
            Some(parse_humansized(min_reserved_str)?)
        } else {
            None
        };

        let max_reserved_size = if let Some(max_reserved_str) = m.value_of("max_reserved_size") {
            Some(parse_humansized(max_reserved_str)?)
        } else {
            None
        };

        let reserved_size = if let Some(reserved_str) = m.value_of("reserved_size") {
            Some(parse_humansized(reserved_str)?)
        } else {
            None
        };

        let guard_size = if let Some(guard_str) = m.value_of("guard_size") {
            Some(parse_humansized(guard_str)?)
        } else {
            None
        };

        let opt_level = match m.value_of("opt_level") {
            None => OptLevel::SpeedAndSize,
            Some("0") | Some("none") => OptLevel::None,
            Some("1") | Some("speed") => OptLevel::Speed,
            Some("2") | Some("speed_and_size") => OptLevel::SpeedAndSize,
            Some(_) => panic!("unknown value for opt-level"),
        };

        let target = match m.value_of("target") {
            None => Triple::host(),
            Some(t) => match Triple::from_str(&t) {
                Ok(triple) => triple,
                Err(_) => panic!("specified target is invalid"),
            },
        };

        let cpu_features = cpu_features_from_args(
            m.value_of("target-cpu"),
            m.values_of("target-feature").unwrap_or_default(),
        )?;

        if target.architecture != Architecture::X86_64 {
            panic!("architectures other than x86-64 are unsupported");
        }

        let keygen = m.is_present("keygen");
        let sign = m.is_present("sign");
        let verify = m.is_present("verify");
        let sk_path = m.value_of("sk_path").map(PathBuf::from);
        let pk_path = m.value_of("pk_path").map(PathBuf::from);
        let count_instructions = m.is_present("count_instructions");

        let error_style = match m.value_of("error_style") {
            None => ErrorStyle::default(),
            Some("human") => ErrorStyle::Human,
            Some("json") => ErrorStyle::Json,
            Some(_) => panic!("unknown value for error-style"),
        };

        Ok(Options {
            output,
            input,
            codegen,
            binding_files,
            witx_specs,
            wasi_exe,
            builtins_path,
            min_reserved_size,
            max_reserved_size,
            reserved_size,
            guard_size,
            opt_level,
            cpu_features,
            keygen,
            sign,
            verify,
            sk_path,
            pk_path,
            count_instructions,
            error_style,
            target,
        })
    }
    pub fn get() -> Result<Self, Error> {
        let _ = include_str!("../Cargo.toml");
        let m = app_from_crate!()
            .arg(
                Arg::with_name("precious")
                    .long("--precious")
                    .takes_value(true)
                    .help("directory to keep intermediate build artifacts in"),
            )
            .arg(
                Arg::with_name("emit")
                    .long("emit")
                    .takes_value(true)
                    .possible_values(&["obj", "so", "clif"])
                    .help("type of code to generate (default: so)"),
            )
            .arg(
                Arg::with_name("output")
                    .short("o")
                    .long("output")
                    .takes_value(true)
                    .multiple(false)
                    .help("output destination, defaults to a.out if unspecified"),
            )
            .arg(
                Arg::with_name("target")
                    .long("target")
                    .takes_value(true)
                    .multiple(false)
                    .help(format!("target to compile for, defaults to {} if unspecified", Triple::host()).as_str()),
            )
            .arg(
                Arg::with_name("target-cpu")
                    .long("--target-cpu")
                    .takes_value(true)
                    .multiple(false)
                    .number_of_values(1)
                    .possible_values(&[
                        "native",
                        "baseline",
                        "nehalem",
                        "sandybridge",
                        "haswell",
                        "broadwell",
                        "skylake",
                        "cannonlake",
                        "icelake",
                        "znver1",
                    ])
                    .help("Generate code for a particular type of CPU.")
                    .long_help(
"Generate code for a particular type of CPU.

If neither `--target-cpu` nor `--target-feature` is provided, `lucetc`
will automatically detect and use the features available on the host CPU.
This is equivalent to choosing `--target-cpu=native`.

"
                    )
            )
            .arg(
                Arg::with_name("target-feature")
                    .long("--target-feature")
                    .takes_value(true)
                    .multiple(true)
                    .use_delimiter(true)
                    .possible_values(&[
                        "+sse3", "-sse3",
                        "+ssse3", "-ssse3",
                        "+sse41", "-sse41",
                        "+sse42", "-sse42",
                        "+popcnt", "-popcnt",
                        "+avx", "-avx",
                        "+bmi1", "-bmi1",
                        "+bmi2", "-bmi2",
                        "+lzcnt", "-lzcnt",
                    ])
                    .help("Enable (+) or disable (-) specific CPU features.")
                    .long_help(
"Enable (+) or disable (-) specific CPU features.

If neither `--target-cpu` nor `--target-feature` is provided, `lucetc`
will automatically detect and use the features available on the host CPU.

This option is additive with, but takes precedence over `--target-cpu`.
For example, `--target-cpu=haswell --target-feature=-avx` will disable
AVX, but leave all other default Haswell features enabled.

Multiple `--target-feature` groups may be specified, with precedence
increasing from left to right. For example, these arguments will enable
SSE3 but not AVX:

    --target-feature=+sse3,+avx --target-feature=-avx

"
                    )
            )
            .arg(
                Arg::with_name("bindings")
                    .long("--bindings")
                    .takes_value(true)
                    .multiple(true)
                    .number_of_values(1)
                    .help("path to bindings json file"),
            )
            .arg(
                Arg::with_name("witx_specs")
                    .long("--witx")
                    .takes_value(true)
                    .multiple(true)
                    .number_of_values(1)
                    .help("path to witx spec to validate against"),
            )
            .arg(
                Arg::with_name("wasi_exe")
                    .long("--wasi_exe")
                    .takes_value(false)
                    .multiple(false)
                    .help("validate as a wasi executable"),
            )
            .arg(
                Arg::with_name("min_reserved_size")
                    .long("--min-reserved-size")
                    .takes_value(true)
                    .multiple(false)
                    .help(&format!(
                        "minimum size of usable linear memory region. must be multiple of 4k. default: {}",
                        humansized(HeapSettings::default().min_reserved_size)
                    )),
            )
            .arg(
                Arg::with_name("max_reserved_size")
                    .long("--max-reserved-size")
                    .takes_value(true)
                    .multiple(false)
                    .help("maximum size of usable linear memory region. must be multiple of 4k. default: 4 GiB"),
            )
            .arg(
                Arg::with_name("reserved_size")
                    .long("--reserved-size")
                    .takes_value(true)
                    .multiple(false)
                    .help("exact size of usable linear memory region, overriding --{min,max}-reserved-size. must be multiple of 4k"),
            )
            .arg(
                Arg::with_name("guard_size")
                    .long("--guard-size")
                    .takes_value(true)
                    .multiple(false)
                    .help(&format!(
                        "size of linear memory guard. must be multiple of 4k. default: {}",
                        humansized(HeapSettings::default().guard_size)
                    )),
            )
            .arg(
                Arg::with_name("builtins")
                    .long("--builtins")
                    .takes_value(true)
                    .help("builtins file"),
            )
            .arg(
                Arg::with_name("input")
                    .multiple(false)
                    .required(false)
                    .help("input file"),
            )
            .arg(
                Arg::with_name("opt_level")
                    .long("--opt-level")
                    .takes_value(true)
                    .possible_values(&["0", "1", "2", "none", "speed", "speed_and_size"])
                    .help("optimization level (default: 'speed_and_size'). 0 is alias to 'none', 1 to 'speed', 2 to 'speed_and_size'"),
            )
            .arg(
                Arg::with_name("keygen")
                    .long("--signature-keygen")
                    .takes_value(false)
                    .help("Create a new key pair")
            )
            .arg(
                Arg::with_name("verify")
                     .long("--signature-verify")
                     .takes_value(false)
                     .help("Verify the signature of the source file")
            )
            .arg(
                Arg::with_name("sign")
                     .long("--signature-create")
                     .takes_value(false)
                     .help("Sign the object file")
            )
            .arg(
                Arg::with_name("pk_path")
                     .long("--signature-pk")
                     .takes_value(true)
                     .help("Path to the public key to verify the source code signature")
            )
            .arg(
                Arg::with_name("sk_path")
                     .long("--signature-sk")
                     .takes_value(true)
                     .help("Path to the secret key to sign the object file. The file can be prefixed with \"raw:\" in order to store a raw, unencrypted secret key")
            )
            .arg(
                Arg::with_name("count_instructions")
                    .long("--count-instructions")
                    .takes_value(false)
                    .help("Instrument the produced binary to count the number of wasm operations the translated program executes")
            )
            .arg(
                Arg::with_name("error_style")
                    .long("error-style")
                    .takes_value(true)
                    .possible_values(&["human", "json"])
                    .help("Style of error reporting (default: human)"),
            )
            .get_matches();

        Self::from_args(&m)
    }
}
