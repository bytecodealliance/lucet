use failure::Fail;
use std::env;
use std::path::PathBuf;
use std::process::{Command, Output};

#[derive(Debug, Fail)]
pub enum CompileError {
    #[fail(display = "File not found: {}", _0)]
    FileNotFound(String),
    #[fail(display = "Clang reported error: {}", _0)]
    Execution { stdout: String, stderr: String },
}

impl CompileError {
    pub fn check(output: Output) -> Result<(), Self> {
        if output.status.success() {
            Ok(())
        } else {
            Err(CompileError::Execution {
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            })
        }
    }
}

fn wasi_sdk_clang() -> PathBuf {
    let mut base = match env::var("WASI_SDK") {
        Ok(sdk) => PathBuf::from(sdk),
        Err(_) => PathBuf::from("/opt/wasi-sdk"),
    };
    base.push("bin");
    base.push("clang");
    base
}

pub struct Compile {
    input: PathBuf,
    cflags: Vec<String>,
}

impl Compile {
    pub fn new(input: PathBuf) -> Self {
        Compile {
            input,
            cflags: Vec::new(),
        }
    }

    pub fn cflag<S: AsRef<str>>(mut self, cflag: S) -> Self {
        self.with_cflag(cflag);
        self
    }

    pub fn with_cflag<S: AsRef<str>>(&mut self, cflag: S) {
        self.cflags.push(cflag.as_ref().to_owned());
    }

    pub fn include<S: AsRef<str>>(mut self, include: S) -> Self {
        self.with_include(include);
        self
    }

    pub fn with_include<S: AsRef<str>>(&mut self, include: S) {
        self.cflags.push(format!("-I{}", include.as_ref()));
    }

    pub fn compile(&self, output: PathBuf) -> Result<(), CompileError> {
        let clang = wasi_sdk_clang();
        if !clang.exists() {
            Err(CompileError::FileNotFound(
                clang.to_string_lossy().into_owned(),
            ))?;
        }
        if !self.input.exists() {
            Err(CompileError::FileNotFound(
                self.input.to_string_lossy().into_owned(),
            ))?;
        }
        let mut cmd = Command::new(clang);
        cmd.arg("-c");
        cmd.arg(self.input.clone());
        cmd.arg("-o");
        cmd.arg(output);
        for cflag in self.cflags.iter() {
            cmd.arg(cflag);
        }
        let run = cmd.output().expect("clang executable exists");
        CompileError::check(run)
    }
}

pub struct Link {
    input: Vec<PathBuf>,
    cflags: Vec<String>,
    ldflags: Vec<String>,
}

impl Link {
    pub fn new(input: Vec<PathBuf>) -> Self {
        Link {
            input,
            cflags: Vec::new(),
            ldflags: Vec::new(),
        }
    }

    pub fn cflag<S: AsRef<str>>(mut self, cflag: S) -> Self {
        self.with_cflag(cflag);
        self
    }

    pub fn with_cflag<S: AsRef<str>>(&mut self, cflag: S) {
        self.cflags.push(cflag.as_ref().to_owned());
    }

    pub fn include<S: AsRef<str>>(mut self, include: S) -> Self {
        self.with_include(include);
        self
    }

    pub fn with_include<S: AsRef<str>>(&mut self, include: S) {
        self.cflags.push(format!("-I{}", include.as_ref()));
    }

    pub fn ldflag<S: AsRef<str>>(mut self, ldflag: S) -> Self {
        self.with_ldflag(ldflag);
        self
    }

    pub fn with_ldflag<S: AsRef<str>>(&mut self, ldflag: S) {
        self.ldflags.push(ldflag.as_ref().to_owned());
    }

    pub fn export<S: AsRef<str>>(mut self, export: S) -> Self {
        self.with_export(export);
        self
    }

    pub fn with_export<S: AsRef<str>>(&mut self, export: S) {
        self.ldflags.push(format!("--export={}", export.as_ref()));
    }

    pub fn link(&self, output: PathBuf) -> Result<(), CompileError> {
        let clang = wasi_sdk_clang();
        if !clang.exists() {
            Err(CompileError::FileNotFound(
                clang.to_string_lossy().into_owned(),
            ))?;
        }
        let mut cmd = Command::new(clang);
        for input in self.input.iter() {
            if !input.exists() {
                Err(CompileError::FileNotFound(
                    input.to_string_lossy().into_owned(),
                ))?;
            }
            cmd.arg(input.clone());
        }
        cmd.arg("-o");
        cmd.arg(output);
        for cflag in self.cflags.iter() {
            cmd.arg(cflag);
        }
        for ldflag in self.ldflags.iter() {
            cmd.arg(format!("-Wl,{}", ldflag));
        }
        let run = cmd.output().expect("clang executable exists");
        CompileError::check(run)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;
    #[test]
    fn wasi_sdk_installed() {
        let clang = wasi_sdk_clang();
        assert!(clang.exists(), "clang executable exists");
    }

    fn test_file(name: &str) -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("test");
        p.push(name);
        assert!(p.exists(), "test file does not exist");
        p
    }

    #[test]
    fn compile_a() {
        let tmp = TempDir::new("compile_a").expect("create temporary directory");

        let compiler = Compile::new(test_file("a.c"));

        let objfile = tmp.path().join("a.o");

        compiler.compile(objfile.clone()).expect("compile a.c");

        assert!(objfile.exists(), "object file created");

        let mut linker = Link::new(vec![objfile]);
        linker.with_cflag("-nostartfiles");
        linker.with_ldflag("--no-entry");

        let wasmfile = tmp.path().join("a.wasm");

        linker.link(wasmfile.clone()).expect("link a.wasm");

        assert!(wasmfile.exists(), "wasm file created");
    }

    #[test]
    fn compile_b() {
        let tmp = TempDir::new("compile_b").expect("create temporary directory");

        let compiler = Compile::new(test_file("b.c"));

        let objfile = tmp.path().join("b.o");

        compiler.compile(objfile.clone()).expect("compile b.c");

        assert!(objfile.exists(), "object file created");

        let mut linker = Link::new(vec![objfile]);
        linker.with_cflag("-nostartfiles");
        linker.with_ldflag("--no-entry");
        linker.with_ldflag("--allow-undefined");

        let wasmfile = tmp.path().join("b.wasm");

        linker.link(wasmfile.clone()).expect("link b.wasm");

        assert!(wasmfile.exists(), "wasm file created");
    }

    #[test]
    fn compile_a_and_b() {
        let tmp = TempDir::new("compile_ab").expect("create temporary directory");

        let mut linker = Link::new(vec![test_file("a.c"), test_file("b.c")]);
        linker.with_cflag("-nostartfiles");
        linker.with_ldflag("--no-entry");

        let wasmfile = tmp.path().join("ab.wasm");

        linker.link(wasmfile.clone()).expect("link ab.wasm");

        assert!(wasmfile.exists(), "wasm file created");
    }
}
