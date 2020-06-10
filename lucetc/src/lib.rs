#![deny(bare_trait_objects)]

mod compiler;
mod decls;
mod error;
mod function;
mod heap;
mod load;
mod module;
mod name;
mod output;
mod pointer;
mod runtime;
pub mod signature;
mod sparsedata;
mod stack_probe;
mod table;
mod traps;
mod types;

use crate::load::read_bytes;
pub use crate::{
    compiler::{Compiler, CompilerBuilder, CpuFeatures, OptLevel, SpecificFeature, TargetCpu},
    error::Error,
    heap::HeapSettings,
    load::read_module,
};
pub use lucet_module::bindings::Bindings;
pub use lucet_validate::Validator;
use signature::{PublicKey, SecretKey};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use target_lexicon::Triple;

enum LucetcInput {
    Bytes(Vec<u8>),
    Path(PathBuf),
}

pub struct Lucetc {
    input: LucetcInput,
    bindings: Vec<Bindings>,
    builder: CompilerBuilder,
    sk: Option<SecretKey>,
    pk: Option<PublicKey>,
    sign: bool,
    verify: bool,
}

pub trait AsLucetc {
    fn as_lucetc(&mut self) -> &mut Lucetc;
}

impl AsLucetc for Lucetc {
    fn as_lucetc(&mut self) -> &mut Lucetc {
        self
    }
}

pub trait LucetcOpts {
    fn bindings(&mut self, bindings: Bindings);
    fn with_bindings(self, bindings: Bindings) -> Self;

    fn target(&mut self, target: Triple);
    fn with_target(self, target: Triple) -> Self;

    fn opt_level(&mut self, opt_level: OptLevel);
    fn with_opt_level(self, opt_level: OptLevel) -> Self;

    fn cpu_features(&mut self, cpu_features: CpuFeatures);
    fn with_cpu_features(self, cpu_features: CpuFeatures) -> Self;

    fn validator(&mut self, validator: Validator);
    fn with_validator(self, validator: Validator) -> Self;

    fn min_reserved_size(&mut self, min_reserved_size: u64);
    fn with_min_reserved_size(self, min_reserved_size: u64) -> Self;

    fn max_reserved_size(&mut self, max_reserved_size: u64);
    fn with_max_reserved_size(self, max_reserved_size: u64) -> Self;

    /// Set the reserved size exactly.
    ///
    /// Equivalent to setting the minimum and maximum reserved sizes to the same value.
    fn reserved_size(&mut self, reserved_size: u64);
    /// Set the reserved size exactly.
    ///
    /// Equivalent to setting the minimum and maximum reserved sizes to the same value.
    fn with_reserved_size(self, reserved_size: u64) -> Self;

    fn guard_size(&mut self, guard_size: u64);
    fn with_guard_size(self, guard_size: u64) -> Self;

    fn pk(&mut self, pk: PublicKey);
    fn with_pk(self, pk: PublicKey) -> Self;
    fn sk(&mut self, sk: SecretKey);
    fn with_sk(self, sk: SecretKey) -> Self;
    fn verify(&mut self);
    fn with_verify(self) -> Self;
    fn sign(&mut self);
    fn with_sign(self) -> Self;
    fn count_instructions(&mut self, enable_count: bool);
    fn with_count_instructions(self, enable_count: bool) -> Self;
    fn canonicalize_nans(&mut self, enable_canonicalize_nans: bool);
    fn with_canonicalize_nans(self, enable_canonicalize_nans: bool) -> Self;
}

impl<T: AsLucetc> LucetcOpts for T {
    fn bindings(&mut self, bindings: Bindings) {
        self.as_lucetc().bindings.push(bindings);
    }

    fn with_bindings(mut self, bindings: Bindings) -> Self {
        self.bindings(bindings);
        self
    }

    fn target(&mut self, target: Triple) {
        self.as_lucetc().builder.target(target);
    }

    fn with_target(mut self, target: Triple) -> Self {
        self.target(target);
        self
    }

    fn opt_level(&mut self, opt_level: OptLevel) {
        self.as_lucetc().builder.opt_level(opt_level);
    }

    fn with_opt_level(mut self, opt_level: OptLevel) -> Self {
        self.opt_level(opt_level);
        self
    }

    fn cpu_features(&mut self, cpu_features: CpuFeatures) {
        self.as_lucetc().builder.cpu_features(cpu_features);
    }

    fn with_cpu_features(mut self, cpu_features: CpuFeatures) -> Self {
        self.cpu_features(cpu_features);
        self
    }

    fn validator(&mut self, validator: Validator) {
        self.as_lucetc().builder.validator(Some(validator));
    }

    fn with_validator(mut self, validator: Validator) -> Self {
        self.validator(validator);
        self
    }

    fn min_reserved_size(&mut self, min_reserved_size: u64) {
        self.as_lucetc()
            .builder
            .heap_settings_mut()
            .min_reserved_size = min_reserved_size;
    }

    fn with_min_reserved_size(mut self, min_reserved_size: u64) -> Self {
        self.min_reserved_size(min_reserved_size);
        self
    }

    fn max_reserved_size(&mut self, max_reserved_size: u64) {
        self.as_lucetc()
            .builder
            .heap_settings_mut()
            .max_reserved_size = max_reserved_size;
    }

    fn with_max_reserved_size(mut self, max_reserved_size: u64) -> Self {
        self.max_reserved_size(max_reserved_size);
        self
    }

    fn reserved_size(&mut self, reserved_size: u64) {
        self.as_lucetc()
            .builder
            .heap_settings_mut()
            .min_reserved_size = reserved_size;
        self.as_lucetc()
            .builder
            .heap_settings_mut()
            .max_reserved_size = reserved_size;
    }

    fn with_reserved_size(mut self, reserved_size: u64) -> Self {
        self.reserved_size(reserved_size);
        self
    }

    fn guard_size(&mut self, guard_size: u64) {
        self.as_lucetc().builder.heap_settings_mut().guard_size = guard_size;
    }

    fn with_guard_size(mut self, guard_size: u64) -> Self {
        self.guard_size(guard_size);
        self
    }

    fn pk(&mut self, pk: PublicKey) {
        self.as_lucetc().pk = Some(pk);
    }

    fn with_pk(mut self, pk: PublicKey) -> Self {
        self.pk(pk);
        self
    }

    fn sk(&mut self, sk: SecretKey) {
        self.as_lucetc().sk = Some(sk);
    }

    fn with_sk(mut self, sk: SecretKey) -> Self {
        self.sk(sk);
        self
    }

    fn verify(&mut self) {
        self.as_lucetc().verify = true;
    }

    fn with_verify(mut self) -> Self {
        self.verify();
        self
    }

    fn sign(&mut self) {
        self.as_lucetc().sign = true;
    }

    fn with_sign(mut self) -> Self {
        self.sign();
        self
    }

    fn count_instructions(&mut self, count_instructions: bool) {
        self.as_lucetc()
            .builder
            .count_instructions(count_instructions);
    }

    fn with_count_instructions(mut self, count_instructions: bool) -> Self {
        self.count_instructions(count_instructions);
        self
    }

    fn canonicalize_nans(&mut self, enable_nans_canonicalization: bool) {
        self.as_lucetc()
            .builder
            .canonicalize_nans(enable_nans_canonicalization);
    }

    fn with_canonicalize_nans(mut self, enable_nans_canonicalization: bool) -> Self {
        self.canonicalize_nans(enable_nans_canonicalization);
        self
    }
}

impl Lucetc {
    pub fn new(input: impl AsRef<Path>) -> Self {
        let input = input.as_ref();
        Self {
            input: LucetcInput::Path(input.to_owned()),
            bindings: vec![],
            builder: Compiler::builder(),
            pk: None,
            sk: None,
            sign: false,
            verify: false,
        }
    }

    pub fn try_from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self, Error> {
        let input = read_bytes(bytes.as_ref().to_vec())?;
        Ok(Self {
            input: LucetcInput::Bytes(input),
            bindings: vec![],
            builder: Compiler::builder(),
            pk: None,
            sk: None,
            sign: false,
            verify: false,
        })
    }

    fn build(&self) -> Result<(Vec<u8>, Bindings), Error> {
        let module_binary = match &self.input {
            LucetcInput::Bytes(bytes) => bytes.clone(),
            LucetcInput::Path(path) => read_module(&path, &self.pk, self.verify)?,
        };

        // Collect set of Bindings into a single Bindings:
        let mut bindings = Bindings::empty();
        for binding in self.bindings.iter() {
            bindings.extend(binding)?;
        }

        Ok((module_binary, bindings))
    }

    pub fn object_file(&self, output: impl AsRef<Path>) -> Result<(), Error> {
        let (module_contents, bindings) = self.build()?;
        let compiler = self.builder.create(&module_contents, &bindings)?;
        let obj = compiler.object_file()?;
        obj.write(output.as_ref())?;

        Ok(())
    }

    pub fn clif_ir(&self, output: impl AsRef<Path>) -> Result<(), Error> {
        let (module_contents, bindings) = self.build()?;

        let compiler = self.builder.create(&module_contents, &bindings)?;

        compiler.cranelift_funcs()?.write(&output)?;

        Ok(())
    }

    pub fn shared_object_file(&self, output: impl AsRef<Path>) -> Result<(), Error> {
        let dir = tempfile::Builder::new().prefix("lucetc").tempdir()?;
        let objpath = dir.path().join("tmp.o");
        self.object_file(objpath.clone())?;
        link_so(objpath, self.builder.target_ref(), &output)?;
        if self.sign {
            let sk = self.sk.as_ref().ok_or(Error::Signature(
                "signing requires a secret key".to_string(),
            ))?;
            signature::sign_module(&output, sk)?;
        }
        Ok(())
    }
}

const LD_DEFAULT: &str = "ld";

fn link_so(
    objpath: impl AsRef<Path>,
    target: &Triple,
    sopath: impl AsRef<Path>,
) -> Result<(), Error> {
    // Let `LD` be something like "clang --target=... ..." for convenience.
    let env_ld = env::var("LD").unwrap_or(LD_DEFAULT.into());
    let mut ld_iter = env_ld.split_whitespace();
    let ld_prog = ld_iter.next().expect("LD must not be empty");
    let mut cmd_ld = Command::new(ld_prog);
    for flag in ld_iter {
        cmd_ld.arg(flag);
    }

    cmd_ld.arg(objpath.as_ref());
    let env_ldflags = env::var("LDFLAGS").unwrap_or_else(|_| ldflags_default(target));
    for flag in env_ldflags.split_whitespace() {
        cmd_ld.arg(flag);
    }

    output_arg_for(&mut cmd_ld, target, sopath);

    let run_ld = cmd_ld.output()?;

    if !run_ld.status.success() {
        let message = format!(
            "ld of {} failed: {}",
            objpath.as_ref().to_str().unwrap(),
            String::from_utf8_lossy(&run_ld.stderr)
        );
        return Err(Error::LdError(message));
    }
    Ok(())
}

fn output_arg_for(cmd_ld: &mut Command, target: &Triple, sopath: impl AsRef<Path>) {
    use target_lexicon::{Environment, OperatingSystem};

    if target.operating_system != OperatingSystem::Windows || target.environment == Environment::Gnu
    {
        cmd_ld.arg("-o");
        cmd_ld.arg(sopath.as_ref());
        return;
    }

    assert!(target.environment == Environment::Msvc);

    cmd_ld.arg(format!("/out:{:?}", sopath.as_ref()));
}

fn ldflags_default(target: &Triple) -> String {
    use target_lexicon::OperatingSystem;

    match target.operating_system {
        OperatingSystem::Linux => "-shared",
        OperatingSystem::Darwin | OperatingSystem::MacOSX { .. } => {
            "-dylib -dead_strip -export_dynamic -undefined dynamic_lookup"
        }
        _ => panic!(
            "Cannot determine default flags for {}.

Please define the LDFLAGS environment variable with the necessary command-line
flags for generating shared libraries.",
            target
        ),
    }
    .into()
}
