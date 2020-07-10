use crate::bindings;
use lucet_runtime::{self, MmapRegion, Module as LucetModule, Region, UntypedRetVal, Val};
use lucetc::{Compiler, CpuFeatures, Error as LucetcError};
use std::io;
use std::process::Command;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("Validation error")]
    ValidationError(#[source] LucetcError),
    #[error("Program error")]
    ProgramError(#[source] LucetcError),
    #[error("Compilation error")]
    CompileError(#[source] LucetcError),
    #[error("Codegen error")]
    CodegenError(#[source] LucetcError),
    #[error("Load error")]
    LoadError(#[source] lucet_runtime::Error),
    #[error("Instantiation error")]
    InstantiateError(#[source] lucet_runtime::Error),
    #[error("Runtime error")]
    RuntimeError(#[source] lucet_runtime::Error),
    #[error("Malformed script: {0}")]
    MalformedScript(String),
    #[error("IO error")]
    IoError(#[from] io::Error),
    #[error("run_ld error: {0}")]
    LdError(String),
}

impl ScriptError {
    pub fn unsupported(&self) -> bool {
        match self {
            ScriptError::ProgramError(ref lucetc_err)
            | ScriptError::ValidationError(ref lucetc_err)
            | ScriptError::CompileError(ref lucetc_err) => match lucetc_err {
                &LucetcError::Unsupported(_) => true,
                _ => false,
            },
            _ => false,
        }
    }
}

pub struct ScriptEnv {
    instances: Vec<(Option<String>, lucet_runtime::InstanceHandle)>,
}

fn program_error(e: LucetcError) -> ScriptError {
    match e {
        LucetcError::WasmValidation(_) => ScriptError::ValidationError(e),
        _ => ScriptError::ProgramError(e),
    }
}

impl ScriptEnv {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
        }
    }
    pub fn instantiate(&mut self, module: &[u8], name: &Option<String>) -> Result<(), ScriptError> {
        let bindings = bindings::spec_test_bindings();
        let builder = Compiler::builder()
            .with_cpu_features(CpuFeatures::baseline())
            .with_count_instructions(true);
        let compiler = builder.create(module, &bindings).map_err(program_error)?;

        let dir = tempfile::Builder::new().prefix("codegen").tempdir()?;
        let objfile_path = dir.path().join("a.o");
        let sofile_path = dir.path().join("a.so");

        compiler
            .object_file()
            .map_err(ScriptError::CompileError)?
            .write(&objfile_path)
            .map_err(ScriptError::CodegenError)?;

        let mut cmd_ld = Command::new("ld");
        cmd_ld.arg(objfile_path.clone());
        cmd_ld.arg("-shared");
        cmd_ld.arg("-o");
        cmd_ld.arg(sofile_path.clone());
        let run_ld = cmd_ld.output()?;
        if !run_ld.status.success() {
            let message = format!(
                "ld {:?}: {}",
                objfile_path,
                String::from_utf8_lossy(&run_ld.stderr)
            );

            return Err(ScriptError::LdError(message));
        }

        let lucet_module: Arc<dyn LucetModule> =
            lucet_runtime::DlModule::load(sofile_path).map_err(ScriptError::LoadError)?;

        let lucet_region = MmapRegion::create(
            1,
            &lucet_runtime::Limits::default().with_heap_memory_size(4 * 1024 * 1024 * 1024),
        )
        .expect("valid region");

        let lucet_instance = lucet_region
            .new_instance(lucet_module.clone())
            .map_err(ScriptError::InstantiateError)?;

        self.instances.push((name.clone(), lucet_instance));
        Ok(())
    }

    fn instance_named_mut(
        &mut self,
        name: &Option<String>,
    ) -> Result<&mut (Option<String>, lucet_runtime::InstanceHandle), ScriptError> {
        Ok(match name {
            // None means the last defined module should be used
            None => self
                .instances
                .last_mut()
                .ok_or_else(|| ScriptError::MalformedScript("no defined instances".to_owned()))?,
            Some(ref n) => self
                .instances
                .iter_mut()
                .find(|(iname, _)| *iname == *name)
                .ok_or_else(|| ScriptError::MalformedScript(format!("no instance named {}", n)))?,
        })
    }

    pub fn instance_named(
        &self,
        name: &Option<String>,
    ) -> Result<&lucet_runtime::InstanceHandle, ScriptError> {
        Ok(match name {
            // None means the last defined module should be used
            None => self
                .instances
                .last()
                .map(|(_fst, snd)| snd)
                .ok_or_else(|| ScriptError::MalformedScript("no defined instances".to_owned()))?,
            Some(ref n) => self
                .instances
                .iter()
                .find(|(iname, _)| *iname == *name)
                .map(|(_fst, snd)| snd)
                .ok_or_else(|| ScriptError::MalformedScript(format!("no instance named {}", n)))?,
        })
    }

    pub fn run(
        &mut self,
        name: &Option<String>,
        field: &str,
        args: Vec<Val>,
    ) -> Result<UntypedRetVal, ScriptError> {
        let (_, ref mut inst) = self.instance_named_mut(name)?;
        inst.run(field, &args)
            .and_then(|rr| rr.returned())
            .map_err(ScriptError::RuntimeError)
    }

    pub fn register(&mut self, name: &Option<String>, as_name: &str) -> Result<(), ScriptError> {
        let (ref mut oldname, _) = self.instance_named_mut(name)?;
        *oldname = Some(as_name.to_owned());
        Ok(())
    }

    pub fn delete_last(&mut self) {
        let last_index = self.instances.len() - 1;
        self.instances.remove(last_index);
    }
}
