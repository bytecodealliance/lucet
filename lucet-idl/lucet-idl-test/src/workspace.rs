use failure::Error;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
pub struct Workspace {
    tempdir: TempDir,
}

impl Workspace {
    pub fn new() -> Result<Self, Error> {
        let tempdir = TempDir::new()?;
        fs::create_dir(tempdir.path().join("src"))?;
        fs::create_dir(tempdir.path().join("out"))?;
        Ok(Self { tempdir })
    }

    pub fn source_path(&self, name: &str) -> PathBuf {
        self.tempdir.path().join("src").join(name)
    }

    pub fn output_path(&self, name: &str) -> PathBuf {
        self.tempdir.path().join("out").join(name)
    }
}
