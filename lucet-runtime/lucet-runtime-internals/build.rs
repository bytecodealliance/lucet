use std::env;
use std::fs::File;
use std::path::Path;

use cc;

fn main() {
    cc::Build::new()
        .file("src/context/context_asm.S")
        .compile("context_context_asm");
    cc::Build::new()
        .file("src/instance/siginfo_ext.c")
        .compile("instance_siginfo_ext");

    // TODO: this should only be built for tests, but Cargo doesn't
    // currently let you specify different build.rs options for tests:
    // <https://github.com/rust-lang/cargo/issues/1581>
    cc::Build::new()
        .file("src/context/tests/c_child.c")
        .compile("context_tests_c_child");

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
