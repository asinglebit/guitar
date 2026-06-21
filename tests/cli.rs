use std::process::Command;

fn run_version_flag(flag: &str) {
    let output = Command::new(env!("CARGO_BIN_EXE_guitar")).arg(flag).output().unwrap();
    assert!(output.status.success(), "binary exited unsuccessfully for {flag}");

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), guitar::VERSION);
    assert!(output.stderr.is_empty(), "unexpected stderr for {flag}: {:?}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn version_flags_print_version_and_exit() {
    for flag in ["--version", "-v"] {
        run_version_flag(flag);
    }
}
