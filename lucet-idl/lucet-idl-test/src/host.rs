use crate::ModuleTestPlan;
use failure::{format_err, Error};
use fs2::FileExt;
use lucet_idl::{self, pretty_writer::PrettyWriter, Backend, Config, Package};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

pub struct HostApp {
    root: PathBuf,
    tempdir: TempDir,
    backups: Vec<(PathBuf, PathBuf)>,
    // lockfile is never used in methods, it just needs to have the same lifetime as the app, it
    // gets unlocked when HostApp drops
    _lockfile: File,
}

impl HostApp {
    pub fn new(package: &Package, test_plan: &ModuleTestPlan) -> Result<Self, Error> {
        let modules = package.modules().collect::<Vec<_>>();
        if modules.len() != 1 {
            Err(format_err!(
                "only one module per package supported at this time"
            ))?
        }

        // Need a system-wide lock on the source directory, because we will modify its contents and
        // call `cargo run` on it.
        // This way we can use the cache of compiled crates in the project cargo workspace.
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("rust_host");
        let lockfile_path = root.join(".rust_host.lock");
        if !lockfile_path.exists() {
            File::create(&lockfile_path)?;
        }

        let lockfile = File::open(lockfile_path)?;
        lockfile.lock_exclusive()?;

        let mut hostapp = HostApp {
            root,
            _lockfile: lockfile,
            tempdir: TempDir::new()?,
            backups: Vec::new(),
        };

        let idl_file = hostapp.source_file("idl.rs")?;
        lucet_idl::codegen(
            package,
            &Config {
                backend: Backend::RustHost,
            },
            Box::new(idl_file),
        )?;

        let mut harness_writer = PrettyWriter::new(Box::new(hostapp.source_file("harness.rs")?));
        test_plan.render_host(&mut harness_writer);

        Ok(hostapp)
    }

    fn source_file(&mut self, name: &str) -> Result<File, Error> {
        let filepath = self.root.join("src").join(name);
        if filepath.exists() {
            let backup = self.tempdir.path().join(name);
            if backup.exists() {
                Err(format_err!(
                    "cannot overwrite source file '{}': already overwritten",
                    name
                ))?
            }
            self.backups.push((backup.clone(), filepath.clone()));
            fs::rename(&filepath, backup)?;
        }
        let f = File::create(filepath)?;
        Ok(f)
    }

    pub fn build(&mut self) -> Result<(), Error> {
        let run_cargo = Command::new("cargo")
            .arg("build")
            .current_dir(&self.root)
            .status()?;
        if !run_cargo.success() {
            Err(format_err!("cargo died building host project"))?
        }
        Ok(())
    }

    pub fn run<P: AsRef<Path>>(&mut self, guest_path: P) -> Result<(), Error> {
        let run_cargo = Command::new("cargo")
            .arg("run")
            .arg("--bin")
            .arg("lucet-idl-test-rust-host")
            .arg("--")
            .arg(guest_path.as_ref())
            .current_dir(&self.root)
            .status()?;
        if !run_cargo.success() {
            Err(format_err!("cargo died building host project"))?
        }
        Ok(())
    }
}

impl Drop for HostApp {
    fn drop(&mut self) {
        for (backup, orig) in self.backups.iter() {
            fs::rename(backup, orig).expect("restore backup")
        }
    }
}
