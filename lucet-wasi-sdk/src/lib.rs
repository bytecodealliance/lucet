#![deny(bare_trait_objects)]

use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;
use thiserror::Error;

const WASI_TARGET: &str = "wasm32-unknown-wasi";

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("Clang reported error: {stdout}")]
    Execution { stdout: String, stderr: String },
    #[error("Lucetc error")]
    Lucetc(#[from] lucetc::Error),
    #[error("IO error")]
    IO(#[from] std::io::Error),
}

impl CompileError {
    pub fn check(output: Output, print: bool) -> Result<(), Self> {
        if print {
            std::io::stdout().write_all(&output.stdout).unwrap();
            std::io::stderr().write_all(&output.stderr).unwrap();
        }
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

fn wasi_sdk() -> PathBuf {
    Path::new(&env::var("WASI_SDK").unwrap_or("/opt/wasi-sdk".to_owned())).to_path_buf()
}

fn wasi_sysroot() -> PathBuf {
    match env::var("WASI_SYSROOT") {
        Ok(wasi_sysroot) => Path::new(&wasi_sysroot).to_path_buf(),
        Err(_) => {
            let mut path = wasi_sdk();
            path.push("share");
            path.push("wasi-sysroot");
            path
        }
    }
}

fn wasm_clang() -> PathBuf {
    match env::var("CLANG") {
        Ok(clang) => Path::new(&clang).to_path_buf(),
        Err(_) => {
            let mut path = wasi_sdk();
            path.push("bin");
            path.push("clang");
            path
        }
    }
}

pub struct Compile {
    input: PathBuf,
    cflags: Vec<String>,
    print_output: bool,
}

pub trait CompileOpts {
    fn cflag<S: AsRef<str>>(&mut self, cflag: S);
    fn with_cflag<S: AsRef<str>>(self, cflag: S) -> Self;

    fn include<S: AsRef<Path>>(&mut self, include: S);
    fn with_include<S: AsRef<Path>>(self, include: S) -> Self;
}

impl CompileOpts for Compile {
    fn cflag<S: AsRef<str>>(&mut self, cflag: S) {
        self.cflags.push(cflag.as_ref().to_owned());
    }

    fn with_cflag<S: AsRef<str>>(mut self, cflag: S) -> Self {
        self.cflag(cflag);
        self
    }

    fn include<S: AsRef<Path>>(&mut self, include: S) {
        self.cflags
            .push(format!("-I{}", include.as_ref().display()));
    }

    fn with_include<S: AsRef<Path>>(mut self, include: S) -> Self {
        self.include(include);
        self
    }
}

impl Compile {
    pub fn new<P: AsRef<Path>>(input: P) -> Self {
        Compile {
            input: PathBuf::from(input.as_ref()),
            cflags: Vec::new(),
            print_output: false,
        }
    }

    pub fn print_output(&mut self, print: bool) {
        self.print_output = print;
    }

    pub fn with_print_output(mut self, print: bool) -> Self {
        self.print_output(print);
        self
    }

    pub fn compile<P: AsRef<Path>>(&self, output: P) -> Result<(), CompileError> {
        let clang = wasm_clang();
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
        cmd.arg(format!("--target={}", WASI_TARGET));
        cmd.arg(format!("--sysroot={}", wasi_sysroot().display()));
        cmd.arg("-c");
        cmd.arg(self.input.clone());
        cmd.arg("-o");
        cmd.arg(output.as_ref());
        for cflag in self.cflags.iter() {
            cmd.arg(cflag);
        }
        if self.print_output {
            println!("running: {:?}", cmd);
        }
        let run = cmd.output().expect("clang executable exists");
        CompileError::check(run, self.print_output)
    }
}

pub struct Link {
    input: Vec<PathBuf>,
    cflags: Vec<String>,
    ldflags: Vec<String>,
    print_output: bool,
}

impl Link {
    pub fn new<P: AsRef<Path>>(input: &[P]) -> Self {
        Link {
            input: input.iter().map(|p| PathBuf::from(p.as_ref())).collect(),
            cflags: vec![],
            ldflags: vec![],
            print_output: false,
        }
        .with_link_opt(LinkOpt::DefaultOpts)
    }

    pub fn print_output(&mut self, print: bool) {
        self.print_output = print;
    }

    pub fn with_print_output(mut self, print: bool) -> Self {
        self.print_output(print);
        self
    }

    pub fn link<P: AsRef<Path>>(&self, output: P) -> Result<(), CompileError> {
        let clang = wasm_clang();
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
        cmd.arg(format!("--target={}", WASI_TARGET));
        cmd.arg(format!("--sysroot={}", wasi_sysroot().display()));
        cmd.arg("-o");
        cmd.arg(output.as_ref());
        for cflag in self.cflags.iter() {
            cmd.arg(cflag);
        }
        for ldflag in self.ldflags.iter() {
            cmd.arg(format!("-Wl,{}", ldflag));
        }
        if self.print_output {
            println!("running: {:?}", cmd);
        }
        let run = cmd.output().expect("clang executable exists");
        CompileError::check(run, self.print_output)
    }
}

pub trait AsLink {
    fn as_link(&mut self) -> &mut Link;
}

impl AsLink for Link {
    fn as_link(&mut self) -> &mut Link {
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub enum LinkOpt<'t> {
    /// Allow references to any undefined function. They will be resolved later by the dynamic linker
    AllowUndefinedAll,

    /// Default options, possibly enabling workarounds for temporary bugs
    DefaultOpts,

    /// Export a symbol
    Export(&'t str),

    /// Preserve all the symbols during LTO, even if they are not used
    ExportAll,

    /// Do not assume that the library has a predefined entry point
    NoDefaultEntryPoint,

    /// Create a shared library
    Shared,

    /// Do not put debug information (STABS or DWARF) in the output file
    StripDebug,

    /// Remove functions and data that are unreachable by the entry point or exported symbols
    StripUnused,
}

impl<'t> LinkOpt<'t> {
    fn as_ldflags(&self) -> Vec<String> {
        match self {
            LinkOpt::AllowUndefinedAll => vec!["--allow-undefined".to_string()],
            LinkOpt::DefaultOpts => vec!["--no-threads".to_string()],
            LinkOpt::Export(symbol) => vec![format!("--export={}", symbol).to_string()],
            LinkOpt::ExportAll => vec!["--export-all".to_string()],
            LinkOpt::NoDefaultEntryPoint => vec!["--no-entry".to_string()],
            LinkOpt::Shared => vec!["--shared".to_string()],
            LinkOpt::StripDebug => vec!["--strip-debug".to_string()],
            LinkOpt::StripUnused => vec!["--strip-discarded".to_string()],
        }
    }
}

pub trait LinkOpts {
    fn link_opt(&mut self, link_opt: LinkOpt<'_>);
    fn with_link_opt(self, link_opt: LinkOpt<'_>) -> Self;

    fn export<S: AsRef<str>>(&mut self, export: S);
    fn with_export<S: AsRef<str>>(self, export: S) -> Self;
}

impl<T: AsLink> LinkOpts for T {
    fn link_opt(&mut self, link_opt: LinkOpt<'_>) {
        self.as_link().ldflags.extend(link_opt.as_ldflags());
    }

    fn with_link_opt(mut self, link_opt: LinkOpt<'_>) -> Self {
        self.link_opt(link_opt);
        self
    }

    fn export<S: AsRef<str>>(&mut self, export: S) {
        self.link_opt(LinkOpt::Export(export.as_ref()));
    }

    fn with_export<S: AsRef<str>>(mut self, export: S) -> Self {
        self.export(export);
        self
    }
}

impl<T: AsLink> CompileOpts for T {
    fn cflag<S: AsRef<str>>(&mut self, cflag: S) {
        self.as_link().cflags.push(cflag.as_ref().to_owned());
    }

    fn with_cflag<S: AsRef<str>>(mut self, cflag: S) -> Self {
        self.cflag(cflag);
        self
    }

    fn include<S: AsRef<Path>>(&mut self, include: S) {
        self.as_link()
            .cflags
            .push(format!("-I{}", include.as_ref().display()));
    }

    fn with_include<S: AsRef<Path>>(mut self, include: S) -> Self {
        self.include(include);
        self
    }
}

pub struct Lucetc {
    link: Link,
    lucetc: lucetc::Lucetc,
    tmpdir: TempDir,
    wasm_file: PathBuf,
}

impl Lucetc {
    pub fn new<P: AsRef<Path>>(input: &[P]) -> Self {
        let link = Link::new(input);
        let tmpdir = TempDir::new().expect("temporary directory creation failed");
        let wasm_file = tmpdir.path().join("out.wasm");
        let lucetc = lucetc::Lucetc::new(&wasm_file);
        Lucetc {
            link,
            lucetc,
            tmpdir,
            wasm_file,
        }
    }

    pub fn print_output(&mut self, print: bool) {
        self.link.print_output = print;
    }

    pub fn with_print_output(mut self, print: bool) -> Self {
        self.print_output(print);
        self
    }

    pub fn build<P: AsRef<Path>>(self, output: P) -> Result<(), CompileError> {
        self.link.link(&self.wasm_file)?;
        self.lucetc
            .shared_object_file(output.as_ref())
            .map_err(CompileError::Lucetc)?;
        Ok(self.tmpdir.close()?)
    }
}

impl AsLink for Lucetc {
    fn as_link(&mut self) -> &mut Link {
        &mut self.link
    }
}

impl lucetc::AsLucetc for Lucetc {
    fn as_lucetc(&mut self) -> &mut lucetc::Lucetc {
        &mut self.lucetc
    }
}

#[test]
fn wasi_sdk_installed() {
    let clang = wasm_clang();
    assert!(clang.exists(), "clang executable exists");
}
