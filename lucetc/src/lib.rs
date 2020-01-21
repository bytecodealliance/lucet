#![deny(bare_trait_objects)]

mod compiler;
mod decls;
mod error;
mod function;
mod function_manifest;
mod heap;
mod load;
mod module;
mod name;
mod output;
mod patch;
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
    compiler::{Compiler, CpuFeatures, OptLevel, SpecificFeature, TargetCpu},
    error::Error,
    heap::HeapSettings,
    load::read_module,
    patch::patch_module,
};
pub use lucet_module::bindings::Bindings;
pub use lucet_validate::Validator;
use signature::{PublicKey, SecretKey};
use std::env;
use std::path::{Path, PathBuf};
use target_lexicon::Triple;
use tempfile;

enum LucetcInput {
    Bytes(Vec<u8>),
    Path(PathBuf),
}

pub struct Lucetc {
    input: LucetcInput,
    bindings: Vec<Bindings>,
    target: Triple,
    opt_level: OptLevel,
    cpu_features: CpuFeatures,
    heap: HeapSettings,
    builtins_paths: Vec<PathBuf>,
    validator: Option<Validator>,
    sk: Option<SecretKey>,
    pk: Option<PublicKey>,
    sign: bool,
    verify: bool,
    count_instructions: bool,
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

    fn builtins<P: AsRef<Path>>(&mut self, builtins_path: P);
    fn with_builtins<P: AsRef<Path>>(self, builtins_path: P) -> Self;

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
        self.as_lucetc().target = target;
    }

    fn with_target(mut self, target: Triple) -> Self {
        self.target(target);
        self
    }

    fn opt_level(&mut self, opt_level: OptLevel) {
        self.as_lucetc().opt_level = opt_level;
    }

    fn with_opt_level(mut self, opt_level: OptLevel) -> Self {
        self.opt_level(opt_level);
        self
    }

    fn cpu_features(&mut self, cpu_features: CpuFeatures) {
        self.as_lucetc().cpu_features = cpu_features;
    }

    fn with_cpu_features(mut self, cpu_features: CpuFeatures) -> Self {
        self.cpu_features(cpu_features);
        self
    }

    fn builtins<P: AsRef<Path>>(&mut self, builtins_path: P) {
        self.as_lucetc()
            .builtins_paths
            .push(builtins_path.as_ref().to_owned());
    }

    fn with_builtins<P: AsRef<Path>>(mut self, builtins_path: P) -> Self {
        self.builtins(builtins_path);
        self
    }

    fn validator(&mut self, validator: Validator) {
        self.as_lucetc().validator = Some(validator);
    }

    fn with_validator(mut self, validator: Validator) -> Self {
        self.validator(validator);
        self
    }

    fn min_reserved_size(&mut self, min_reserved_size: u64) {
        self.as_lucetc().heap.min_reserved_size = min_reserved_size;
    }

    fn with_min_reserved_size(mut self, min_reserved_size: u64) -> Self {
        self.min_reserved_size(min_reserved_size);
        self
    }

    fn max_reserved_size(&mut self, max_reserved_size: u64) {
        self.as_lucetc().heap.max_reserved_size = max_reserved_size;
    }

    fn with_max_reserved_size(mut self, max_reserved_size: u64) -> Self {
        self.max_reserved_size(max_reserved_size);
        self
    }

    fn reserved_size(&mut self, reserved_size: u64) {
        self.as_lucetc().heap.min_reserved_size = reserved_size;
        self.as_lucetc().heap.max_reserved_size = reserved_size;
    }

    fn with_reserved_size(mut self, reserved_size: u64) -> Self {
        self.reserved_size(reserved_size);
        self
    }

    fn guard_size(&mut self, guard_size: u64) {
        self.as_lucetc().heap.guard_size = guard_size;
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
        self.as_lucetc().count_instructions = count_instructions;
    }

    fn with_count_instructions(mut self, count_instructions: bool) -> Self {
        self.count_instructions(count_instructions);
        self
    }
}

impl Lucetc {
    pub fn new<P: AsRef<Path>>(input: P) -> Self {
        let input = input.as_ref();
        Self {
            input: LucetcInput::Path(input.to_owned()),
            bindings: vec![],
            target: Triple::host(),
            opt_level: OptLevel::default(),
            cpu_features: CpuFeatures::default(),
            heap: HeapSettings::default(),
            builtins_paths: vec![],
            validator: None,
            pk: None,
            sk: None,
            sign: false,
            verify: false,
            count_instructions: false,
        }
    }

    pub fn try_from_bytes<B: AsRef<[u8]>>(bytes: B) -> Result<Self, Error> {
        let input = read_bytes(bytes.as_ref().to_vec())?;
        Ok(Self {
            input: LucetcInput::Bytes(input),
            bindings: vec![],
            target: Triple::host(),
            opt_level: OptLevel::default(),
            cpu_features: CpuFeatures::default(),
            heap: HeapSettings::default(),
            builtins_paths: vec![],
            validator: None,
            pk: None,
            sk: None,
            sign: false,
            verify: false,
            count_instructions: false,
        })
    }

    fn build(&self) -> Result<(Vec<u8>, Bindings), Error> {
        use parity_wasm::elements::{deserialize_buffer, serialize};

        let mut builtins_bindings = vec![];
        let mut module_binary = match &self.input {
            LucetcInput::Bytes(bytes) => bytes.clone(),
            LucetcInput::Path(path) => read_module(&path, &self.pk, self.verify)?,
        };

        if !self.builtins_paths.is_empty() {
            let mut module = deserialize_buffer(&module_binary)?;
            for builtins in self.builtins_paths.iter() {
                let (newmodule, builtins_map) = patch_module(module, builtins)?;
                module = newmodule;
                builtins_bindings.push(Bindings::env(builtins_map));
            }
            module_binary = serialize(module)?;
        }

        let mut bindings = Bindings::empty();

        for binding in builtins_bindings.iter().chain(self.bindings.iter()) {
            bindings.extend(binding)?;
        }

        Ok((module_binary, bindings))
    }

    pub fn object_file<P: AsRef<Path>>(&self, output: P) -> Result<(), Error> {
        let (module_contents, bindings) = self.build()?;

        let compiler = Compiler::new(
            &module_contents,
            self.target.clone(),
            self.opt_level,
            self.cpu_features.clone(),
            &bindings,
            self.heap.clone(),
            self.count_instructions,
            &self.validator,
        )?;
        let obj = compiler.object_file()?;
        obj.write(output.as_ref())?;

        Ok(())
    }

    pub fn clif_ir<P: AsRef<Path>>(&self, output: P) -> Result<(), Error> {
        let (module_contents, bindings) = self.build()?;

        let compiler = Compiler::new(
            &module_contents,
            self.target.clone(),
            self.opt_level,
            self.cpu_features.clone(),
            &bindings,
            self.heap.clone(),
            self.count_instructions,
            &self.validator,
        )?;

        compiler.cranelift_funcs()?.write(&output)?;

        Ok(())
    }

    pub fn shared_object_file<P: AsRef<Path>>(&self, output: P) -> Result<(), Error> {
        let dir = tempfile::Builder::new().prefix("lucetc").tempdir()?;
        let objpath = dir.path().join("tmp.o");
        self.object_file(objpath.clone())?;
        link_so(objpath, &self.target, &output)?;
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

fn link_so<P, Q>(objpath: P, target: &Triple, sopath: Q) -> Result<(), Error>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    use std::process::Command;
    let mut cmd_ld = Command::new(env::var("LD").unwrap_or(LD_DEFAULT.into()));
    cmd_ld.arg(objpath.as_ref());
    let env_ldflags = env::var("LDFLAGS").unwrap_or_else(|_| ldflags_default(target));
    for flag in env_ldflags.split_whitespace() {
        cmd_ld.arg(flag);
    }
    cmd_ld.arg("-o");
    cmd_ld.arg(sopath.as_ref());

    let run_ld = cmd_ld.output()?;

    if !run_ld.status.success() {
        let message = format!(
            "ld of {} failed: {}",
            objpath.as_ref().to_str().unwrap(),
            String::from_utf8_lossy(&run_ld.stderr)
        );
        Err(Error::LdError(message))?;
    }
    Ok(())
}

fn ldflags_default(target: &Triple) -> String {
    use target_lexicon::OperatingSystem;

    match target.operating_system {
        OperatingSystem::Linux => "-shared",
        OperatingSystem::MacOSX { .. } => {
            "-dylib -dead_strip -export_dynamic -undefined dynamic_lookup"
        }
        _ => panic!(
            "Cannot determine default flags for {}.

Please define the LDFLAGS environment variable with the necessary command-line
flags for generating shared libraries.",
            Triple::host()
        ),
    }
    .into()
}
