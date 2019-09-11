use std::env;
use std::fs::File;
use std::path::Path;

fn main() {
    let commit_file_path = Path::new(&env::var("OUT_DIR").unwrap()).join("commit_hash");
    // in debug builds we only need the file to exist, but in release builds this will be used and
    // requires mutability.
    #[allow(unused_variables, unused_mut)]
    let mut f = File::create(&commit_file_path).unwrap();

    // This is about the closest not-additional-feature-flag way to detect release builds.
    // In debug builds, leave the `commit_hash` file empty to allow looser version checking and
    // avoid impacting development workflows too much.
    #[cfg(not(debug_assertions))]
    {
        use std::io::Write;
        use std::process::Command;

        let last_commit_hash = Command::new("git")
            .args(&["log", "-n", "1", "--pretty=format:%H"])
            .output()
            .ok();

        if let Some(last_commit_hash) = last_commit_hash {
            f.write_all(&last_commit_hash.stdout).unwrap();
        }
    }
}
