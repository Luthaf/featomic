use std::path::PathBuf;
use std::process::Command;

mod utils;

#[test]
fn check_python() {
    utils::check_command_exists("tox");

    let mut root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    root.pop();

    let mut tox = Command::new("tox");
    tox.arg("--");
    if cfg!(debug_assertions) {
        // assume that debug assertion means that we are building the code in
        // debug mode, even if that could be not true in some cases
        tox.env("RASCALINE_BUILD_TYPE", "debug");
    } else {
        tox.env("RASCALINE_BUILD_TYPE", "release");
    }
    tox.current_dir(&root);
    let status = tox.status().expect("failed to run tox");
    assert!(status.success());
}
