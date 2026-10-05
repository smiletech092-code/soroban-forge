//! End-to-end behaviour of `soroban-forge spec` that does not need a
//! `stellar` binary: everything up to the point the interface would be read
//! out of a built wasm.

use std::process::Command;

fn forge() -> Command {
    Command::new(env!("CARGO_BIN_EXE_soroban-forge"))
}

/// Scaffold a project with `new` and return its path.
fn scaffold(parent: &std::path::Path, name: &str) -> std::path::PathBuf {
    let output = forge()
        .args([
            "--quiet",
            "new",
            name,
            "--template",
            "hello-world",
            "--author",
            "Test Author",
            "--no-git",
            "--output-dir",
            parent.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    parent.join(name)
}

#[test]
fn spec_is_listed_as_a_subcommand() {
    let output = forge().arg("--list").output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("spec"), "{stdout}");
}

#[test]
fn spec_help_mentions_entrypoints_and_types() {
    let output = forge().args(["spec", "--help"]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("entrypoint"), "{stdout}");
    assert!(stdout.contains("--wasm"), "{stdout}");
    assert!(stdout.contains("--path"), "{stdout}");
}

#[test]
fn spec_without_a_build_points_at_stellar_contract_build() {
    let temp = tempfile::tempdir().unwrap();
    let project = scaffold(temp.path(), "spec-demo");

    let output = forge()
        .args(["spec", "--path", project.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("stellar contract build"), "{stderr}");
    assert!(stderr.contains("spec_demo.wasm"), "{stderr}");
}

#[test]
fn spec_outside_a_cargo_project_says_so() {
    let temp = tempfile::tempdir().unwrap();
    let output = forge()
        .args(["spec", "--path", temp.path().to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("not a cargo project"), "{stderr}");
}

#[test]
fn quiet_spec_keeps_stdout_empty_on_failure() {
    let temp = tempfile::tempdir().unwrap();
    let project = scaffold(temp.path(), "quiet-spec-demo");

    let output = forge()
        .args(["--quiet", "spec", "--path", project.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
}

#[test]
fn json_spec_reports_errors_as_json() {
    let temp = tempfile::tempdir().unwrap();
    let project = scaffold(temp.path(), "json-spec-demo");

    let output = forge()
        .args(["--json", "spec", "--path", project.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stderr).expect("stderr must be JSON");
    assert_eq!(parsed["exit_code"], 1);
    assert!(parsed["error"]
        .as_str()
        .unwrap()
        .contains("stellar contract build"));
}

#[test]
fn spec_help_mentions_contract_id_and_format() {
    let output = forge().args(["spec", "--help"]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("CONTRACT_ID"), "{stdout}");
    assert!(stdout.contains("--format"), "{stdout}");
    assert!(stdout.contains("--network"), "{stdout}");
}

#[test]
fn spec_rejects_malformed_contract_id_before_network() {
    let output = forge()
        .args([
            "spec",
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("not a valid contract ID"), "{stderr}");
}

#[test]
fn spec_with_contract_id_refuses_offline() {
    let output = forge()
        .args([
            "--offline",
            "spec",
            "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("offline mode"), "{stderr}");
}

// ---------------------------------------------------------------------------
// #399 — --no-cache flag is accepted by the CLI
// ---------------------------------------------------------------------------

#[test]
fn spec_help_mentions_no_cache() {
    let output = forge().args(["spec", "--help"]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--no-cache"), "{stdout}");
}

#[test]
fn spec_no_cache_flag_is_accepted() {
    // Without a real wasm the command will fail at the "no built wasm" stage,
    // but the flag itself must be recognised (no "unexpected argument" error).
    let temp = tempfile::tempdir().unwrap();
    let project = scaffold(temp.path(), "no-cache-demo");

    let output = forge()
        .args([
            "spec",
            "--path",
            project.to_str().unwrap(),
            "--no-cache",
        ])
        .output()
        .unwrap();

    // Fails because there is no built wasm — but NOT because --no-cache is unknown.
    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        !stderr.contains("unexpected argument"),
        "--no-cache should be a recognised flag, got: {stderr}"
    );
    assert!(stderr.contains("stellar contract build"), "{stderr}");
}

// ---------------------------------------------------------------------------
// #401 — --count flag is accepted by the CLI
// ---------------------------------------------------------------------------

#[test]
fn spec_help_mentions_count() {
    let output = forge().args(["spec", "--help"]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--count"), "{stdout}");
}

#[test]
fn spec_count_flag_is_accepted() {
    // Same as no-cache: fails because of no wasm, but --count must be recognised.
    let temp = tempfile::tempdir().unwrap();
    let project = scaffold(temp.path(), "count-demo");

    let output = forge()
        .args([
            "spec",
            "--path",
            project.to_str().unwrap(),
            "--count",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        !stderr.contains("unexpected argument"),
        "--count should be a recognised flag, got: {stderr}"
    );
    assert!(stderr.contains("stellar contract build"), "{stderr}");
}

// ---------------------------------------------------------------------------
// #402 — --timeout is surfaced in spec help / accepted as a global flag
// ---------------------------------------------------------------------------

#[test]
fn spec_global_timeout_flag_is_in_help() {
    // The global --timeout flag is defined at the root command level.
    let output = forge().arg("--help").output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--timeout"), "{stdout}");
}

#[test]
fn spec_with_timeout_flag_accepted() {
    // Passes --timeout 30 before spec; command should fail at missing-wasm,
    // not at "unrecognised argument".
    let temp = tempfile::tempdir().unwrap();
    let project = scaffold(temp.path(), "timeout-spec-demo");

    let output = forge()
        .args([
            "--timeout",
            "30",
            "spec",
            "--path",
            project.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        !stderr.contains("unexpected argument"),
        "--timeout should be a recognised global flag, got: {stderr}"
    );
    assert!(stderr.contains("stellar contract build"), "{stderr}");
}

// #403 — spec --out: help must mention --out and --force
#[test]
fn spec_help_mentions_out_and_force_flags() {
    let output = forge().args(["spec", "--help"]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--out"), "help must mention --out: {stdout}");
    assert!(stdout.contains("--force"), "help must mention --force: {stdout}");
}

// #403 — spec --out: without --force, must refuse to overwrite an existing file
#[test]
fn spec_out_refuses_to_overwrite_existing_file_without_force() {
    let temp = tempfile::tempdir().unwrap();
    let project = scaffold(temp.path(), "spec-out-no-force");
    let out_file = temp.path().join("spec.txt");

    // Pre-create the output file
    std::fs::write(&out_file, "existing content").unwrap();

    let output = forge()
        .args([
            "spec",
            "--path",
            project.to_str().unwrap(),
            "--out",
            out_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    // Should report AlreadyExists (exit code 1)
    assert!(
        stderr.contains("already exists") || output.status.code() == Some(1),
        "expected already-exists error: {stderr}"
    );
    // Original file must be untouched
    assert_eq!(
        std::fs::read_to_string(&out_file).unwrap(),
        "existing content"
    );
}
