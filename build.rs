
#[cfg(target_os = "windows")]
extern crate embed_resource;

use std::process::Command;

fn main() {
    let output = Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .unwrap();
    let git_branch = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_BRANCH={}", git_branch);

    #[cfg(target_os = "windows")]
    embed_resource::compile("browser-selector-rt.rc");
}
