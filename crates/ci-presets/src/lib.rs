//! # soroban-forge-ci-presets
//!
//! `soroban-forge ci-init --provider github` — writes CI/CD workflows for a
//! Soroban contract project.

use clap::{Arg, ArgAction, ArgMatches, Command};
use include_dir::{include_dir, Dir};
use serde::Deserialize;
use soroban_forge_core::render::{render_str, Vars};
use soroban_forge_core::{ForgeContext, ForgeError, ForgePlugin, Result};
use std::io::Write;
use std::path::Path;

static PRESETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../presets");

const BASE_WORKFLOWS: &[&str] = &["build-test.yml", "contract-size.yml"];
const MATRIX_WORKFLOW: &str = "build-test-matrix.yml";
const DEPLOY_WORKFLOW: &str = "testnet-deploy.yml";
const RELEASE_WORKFLOW: &str = "release.yml";
const SECURITY_SCAN_WORKFLOW: &str = "security-scan.yml";
const COVERAGE_WORKFLOW: &str = "coverage.yml";
const ACTIONLINT_WORKFLOW: &str = "actionlint.yml";
const DENY_TOML: &str = "deny.toml";
const HEALTHCHECK_WORKFLOW: &str = "testnet-healthcheck.yml";
const STALE_WORKFLOW: &str = "stale.yml";
const STEP_MARKER: &str = "# ci-init-step: ";
const STEP_OVERRIDES_DIR: &str = ".soroban-forge/ci-overrides/github";
pub const DEFAULT_MSRV: &str = "1.84";
pub const DEFAULT_MAX_SIZE: u64 = 65_536;
const DEPENDABOT_CONFIG: &str = "dependabot.yml";

pub fn available_providers() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = PRESETS
        .dirs()
        .filter_map(|d| d.path().file_name().and_then(|n| n.to_str()))
        .collect();
    names.sort_unstable();
    names
}

pub fn output_dir(provider: &str) -> &'static str {
    match provider {
        "github" => ".github/workflows",
        "gitlab" | "bitbucket" | "azure" | "woodpecker" => ".",
        "circleci" => ".circleci",
        "buildkite" => ".buildkite",
        _ => unreachable!("validated against available_providers()"),
    }
}

#[derive(Deserialize)]
struct Manifest {
    package: Package,
}

#[derive(Deserialize)]
struct Package {
    name: String,
}

fn project_name(dir: &Path, ctx: &ForgeContext) -> String {
    if let Some(name) = ctx.config.as_ref().and_then(|c| c.project.name.clone()) {
        return name;
    }
    let manifest_path = dir.join("Cargo.toml");
    if let Ok(raw) = std::fs::read_to_string(&manifest_path) {
        if let Ok(manifest) = toml::from_str::<Manifest>(&raw) {
            return manifest.package.name;
        }
    }
    dir.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "contract".to_string())
}

fn render_preset(dir: &Path, provider: &str, contents: &str, vars: &Vars) -> Result<String> {
    let rendered = render_str(contents, vars);
    if provider != "github" || !rendered.contains(STEP_MARKER) {
        return Ok(rendered);
    }

    let lines: Vec<&str> = rendered.split_inclusive('\n').collect();
    let mut output = String::with_capacity(rendered.len());
    let mut index = 0;
    while index < lines.len() {
        let Some(marker) = lines[index].trim_start().strip_prefix(STEP_MARKER) else {
            output.push_str(lines[index]);
            index += 1;
            continue;
        };

        let marker = marker.trim();
        let mut step_index = index + 1;
        while step_index < lines.len() && lines[step_index].trim().is_empty() {
            step_index += 1;
        }
        if step_index == lines.len() {
            return Err(ForgeError::Template(format!(
                "step marker `{marker}` has no step"
            )));
        }

        let step_indent = lines[step_index].len() - lines[step_index].trim_start().len();
        if !lines[step_index][step_indent..].starts_with("- ") {
            return Err(ForgeError::Template(format!(
                "step marker `{marker}` is not followed by a YAML step"
            )));
        }

        let mut end = step_index + 1;
        while end < lines.len() {
            let line = lines[end];
            if !line.trim().is_empty() {
                let indent = line.len() - line.trim_start().len();
                if indent <= step_indent {
                    break;
                }
            }
            end += 1;
        }

        let override_path = dir.join(format!("{STEP_OVERRIDES_DIR}/{marker}.yml"));
        if override_path.exists() {
            let override_contents = std::fs::read_to_string(&override_path)
                .map_err(ForgeError::io(format!("reading {}", override_path.display())))?;
            let override_rendered = render_str(&override_contents, vars);
            output.push_str(&indent_step(&override_rendered, step_indent, marker)?);
        } else {
            for line in &lines[step_index..end] {
                output.push_str(line);
            }
        }
        index = end;
    }
    Ok(output)
}

fn indent_step(step: &str, indent: usize, marker: &str) -> Result<String> {
    let lines: Vec<&str> = step.lines().collect();
    let baseline = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);
    let first = lines
        .iter()
        .find(|line| !line.trim().is_empty())
        .ok_or_else(|| ForgeError::Template(format!("step override `{marker}` is empty")))?;
    if !first[baseline..].starts_with("- ") {
        return Err(ForgeError::Template(format!(
            "step override `{marker}` must contain one YAML list item"
        )));
    }

    let prefix = " ".repeat(indent);
    let mut output = String::new();
    for line in lines {
        if line.trim().is_empty() {
            output.push('\n');
        } else {
            output.push_str(&prefix);
            output.push_str(&line[baseline..]);
            output.push('\n');
        }
    }
    Ok(output)
}

#[derive(Debug, Default, Clone)]
pub struct GenerateOptions {
    pub deploy: bool,
    pub security_scan: bool,
    pub coverage: bool,
    pub actionlint: bool,
    pub healthcheck: bool,
    pub matrix: bool,
    pub stale: bool,
    pub msrv: Option<String>,
    pub dependabot: bool,
    pub max_size: Option<u64>,
}

pub fn generate(
    dir: &Path,
    provider: &str,
    project_name: &str,
    _deploy: bool,
    release: bool,
    opts: &GenerateOptions,
    force: bool,
) -> Result<Vec<String>> {
    let provider_dir = PRESETS.get_dir(provider).ok_or_else(|| {
        ForgeError::InvalidArgument(format!(
            "unknown provider `{provider}` (available: {})",
            available_providers().join(", ")
        ))
    })?;

    let mut vars = Vars::new();
    let max_size = opts.max_size.unwrap_or(DEFAULT_MAX_SIZE);
    vars.insert("project_name".into(), project_name.to_string());
    vars.insert("crate_name".into(), project_name.replace('-', "_"));
    vars.insert(
        "msrv".into(),
        opts.msrv
            .clone()
            .unwrap_or_else(|| DEFAULT_MSRV.to_string()),
    );
    vars.insert("max_size".into(), max_size.to_string());

    let out_rel = output_dir(provider);
    let out_dir = dir.join(out_rel);
    std::fs::create_dir_all(&out_dir)
        .map_err(ForgeError::io(format!("creating {}", out_dir.display())))?;

    let mut selected: Vec<(&str, Option<&str>)> = match provider {
        "github" => {
            let mut list: Vec<(&str, Option<&str>)> =
                BASE_WORKFLOWS.iter().map(|n| (*n, None)).collect();
            if opts.deploy {
                list.push((DEPLOY_WORKFLOW, None));
            }
            if opts.security_scan {
                list.push((SECURITY_SCAN_WORKFLOW, None));
                list.push((DENY_TOML, Some(".")));
            }
            if opts.coverage {
                list.push((COVERAGE_WORKFLOW, None));
            }
            if opts.actionlint {
                list.push((ACTIONLINT_WORKFLOW, None));
            }
            if opts.healthcheck {
                list.push((HEALTHCHECK_WORKFLOW, None));
            }
            if opts.stale {
                list.push((STALE_WORKFLOW, None));
            }
            if release {
                list.push((RELEASE_WORKFLOW, None));
            }
            if opts.matrix {
                list.push((MATRIX_WORKFLOW, None));
            }
            if opts.dependabot {
                list.push((DEPENDABOT_CONFIG, Some(".github")));
            }
            list
        }
        "gitlab" => vec![(".gitlab-ci.yml", None)],
        "bitbucket" => vec![("bitbucket-pipelines.yml", None)],
        "azure" => vec![("azure-pipelines.yml", None)],
        "circleci" => vec![("config.yml", None)],
        "woodpecker" => vec![(".woodpecker.yml", None)],
        "buildkite" => vec![("pipeline.yml", None)],
        _ => {
            return Err(ForgeError::InvalidArgument(format!(
                "unknown provider `{provider}` (available: {})",
                available_providers().join(", ")
            )));
        }
    };

    let mut written = Vec::new();
    for (name, dest_rel_override) in selected.drain(..) {
        let file = provider_dir
            .get_file(format!("{provider}/{name}"))
            .ok_or_else(|| {
                ForgeError::Template(format!("missing preset file {provider}/{name}"))
            })?;
        let contents = file
            .contents_utf8()
            .ok_or_else(|| ForgeError::Template(format!("preset {name} is not UTF-8")))?;

        let dest_rel = dest_rel_override.unwrap_or(out_rel);
        let dest_dir = dir.join(dest_rel);
        std::fs::create_dir_all(&dest_dir)
            .map_err(ForgeError::io(format!("creating {}", dest_dir.display())))?;
        let out_path = dest_dir.join(name);
        if out_path.exists() && !force {
            return Err(ForgeError::AlreadyExists(out_path));
        }
        std::fs::write(&out_path, render_preset(dir, provider, contents, &vars)?)
            .map_err(ForgeError::io(format!("writing {}", out_path.display())))?;

        let rel_path = if dest_rel == "." {
            name.to_string()
        } else {
            format!("{dest_rel}/{name}")
        };
        written.push(rel_path);
    }
    Ok(written)
}

pub fn format_report(
    provider: &str,
    name: &str,
    written: &[impl AsRef<str>],
    release: bool,
    opts: &GenerateOptions,
) -> String {
    let mut out = format!("wrote {provider} workflows for `{name}`:\n");
    for path in written {
        out.push_str(&format!("  {}\n", path.as_ref()));
    }
    if opts.deploy {
        let secret_kind = if provider == "gitlab" {
            "GitLab CI/CD variable"
        } else {
            "GitHub secret"
        };
        let secret_location = if provider == "gitlab" {
            "  repo -> Settings -> CI/CD -> Variables\n"
        } else {
            "  repo -> Settings -> Secrets and variables -> Actions\n"
        };
        out.push_str(&format!(
            "\nthe deploy workflow needs a {secret_kind} named STELLAR_DEPLOYER_SECRET\n\
             (a funded testnet account's secret key). Add it under:\n\
             {secret_location}"
        ));
    }
    if opts.security_scan {
        out.push_str(
            "\nsecurity-scan: review deny.toml at the project root to tune license and\n\
             vulnerability policies. Install locally with: cargo install cargo-audit cargo-deny\n",
        );
    }
    if opts.dependabot {
        out.push_str(
            "\ndependabot: weekly update PRs for the cargo and github-actions ecosystems.\n\
             Enable Dependabot under: repo -> Settings -> Code security\n",
        );
    }
    if opts.coverage {
        out.push_str(
            "\ncoverage: upload test results to Codecov by adding a repository secret named\n\
             CODECOV_TOKEN. If that secret is missing, the workflow skips the upload gracefully.\n",
        );
    }
    if opts.actionlint {
        out.push_str(
            "\nactionlint: validate generated GitHub workflows in CI with `actionlint` before they merge.\n",
        );
    }
    if opts.healthcheck {
        out.push_str(
            "\ntestnet-healthcheck: the smoke entry point defaults to `version` then `ping`.\n\
             Edit testnet-healthcheck.yml to invoke your contract's real health method.\n",
        );
    }
    if opts.stale {
        out.push_str(
            "\nstale: marks inactive issues and PRs stale, and closes them after the configured grace period.\n\
             The workflow inputs can be adjusted from GitHub Actions -> stale -> Run workflow.\n",
        );
    }
    if opts.matrix {
        let msrv = opts.msrv.as_deref().unwrap_or(DEFAULT_MSRV);
        out.push_str(&format!(
            "\nbuild-test-matrix: the job runs once per toolchain — stable and MSRV {msrv}.\n\
             Pass --msrv <version> to pin a different MSRV, and keep it in sync with\n\
             `rust-version` in Cargo.toml.\n"
        ));
    }
    if release {
        out.push_str("\npush a tag matching `v*.*.*` (e.g. `v0.1.0`) to build the wasm,\n");
        out.push_str("verify the build is reproducible, and publish it to a GitHub Release\n");
        out.push_str(
            "with a SHA256 checksum and detached signatures. Uses the default GITHUB_TOKEN — no secrets needed.\n",
        );
    }
    out
}

fn print_diff(path: &Path, generated: &str) -> Result<()> {
    let temp_path = std::env::temp_dir().join(format!(
        "soroban-forge-ci-init-{}-{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let mut temp_file = std::fs::File::create(&temp_path)
        .map_err(ForgeError::io(format!("creating {}", temp_path.display())))?;
    temp_file
        .write_all(generated.as_bytes())
        .map_err(ForgeError::io(format!("writing {}", temp_path.display())))?;

    let left = if path.exists() { path } else { Path::new("/dev/null") };
    let mut cmd = std::process::Command::new("diff");
    cmd.arg("-u").arg("--label").arg("generated").arg("--label").arg(path.to_string_lossy().as_ref());
    cmd.arg(left).arg(&temp_path);
    let output = cmd.output().map_err(ForgeError::io(format!("running diff for {}", path.display())))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("\n--- {} ---\n{stdout}", path.display());
    std::fs::remove_file(&temp_path).ok();
    Ok(())
}

pub struct CiPresetsPlugin;

impl ForgePlugin for CiPresetsPlugin {
    fn name(&self) -> &'static str {
        "ci-init"
    }

    fn command(&self) -> Command {
        Command::new("ci-init")
            .about("Write CI/CD workflows")
            .arg(
                Arg::new("provider")
                    .long("provider")
                    .default_value("github")
                    .help("CI provider (`github`, `gitlab`, `circleci`, `azure`, `bitbucket`, `woodpecker`, or `buildkite`)"),
            )
            .arg(Arg::new("deploy").long("deploy").action(ArgAction::SetTrue))
            .arg(Arg::new("security-scan").long("security-scan").action(ArgAction::SetTrue))
            .arg(Arg::new("coverage").long("coverage").action(ArgAction::SetTrue))
            .arg(Arg::new("actionlint").long("actionlint").action(ArgAction::SetTrue))
            .arg(Arg::new("healthcheck").long("healthcheck").action(ArgAction::SetTrue))
            .arg(Arg::new("matrix").long("matrix").action(ArgAction::SetTrue))
            .arg(Arg::new("stale").long("stale").action(ArgAction::SetTrue))
            .arg(Arg::new("msrv").long("msrv").value_name("VERSION"))
            .arg(Arg::new("max-size").long("max-size").value_name("BYTES").value_parser(clap::value_parser!(u64)))
            .arg(Arg::new("dependabot").long("dependabot").action(ArgAction::SetTrue))
            .arg(Arg::new("release").long("release").action(ArgAction::SetTrue))
            .arg(Arg::new("diff").long("diff").action(ArgAction::SetTrue))
            .arg(Arg::new("path").long("path"))
            .arg(Arg::new("force").long("force").action(ArgAction::SetTrue))
    }

    fn run(&self, matches: &ArgMatches, ctx: &ForgeContext) -> Result<()> {
        let provider = matches.get_one::<String>("provider").expect("has default");
        let dir = matches
            .get_one::<String>("path")
            .map(|p| ctx.cwd.join(p))
            .unwrap_or_else(|| ctx.cwd.clone());
        let name = project_name(&dir, ctx);
        let max_size = matches
            .get_one::<u64>("max-size")
            .copied()
            .or_else(|| {
                ctx.config.as_ref().and_then(|c| {
                    c.defaults
                        .ci_init
                        .max_size
                })
            })
            .unwrap_or(DEFAULT_MAX_SIZE);
        let opts = GenerateOptions {
            deploy: matches.get_flag("deploy"),
            security_scan: matches.get_flag("security-scan"),
            coverage: matches.get_flag("coverage"),
            actionlint: matches.get_flag("actionlint"),
            healthcheck: matches.get_flag("healthcheck"),
            matrix: matches.get_flag("matrix"),
            stale: matches.get_flag("stale"),
            msrv: matches.get_one::<String>("msrv").cloned(),
            dependabot: matches.get_flag("dependabot"),
            max_size: Some(max_size),
        };

        if matches.get_flag("diff") {
            let provider_dir = PRESETS.get_dir(provider).ok_or_else(|| {
                ForgeError::InvalidArgument(format!(
                    "unknown provider `{provider}` (available: {})",
                    available_providers().join(", ")
                ))
            })?;
            let mut selected: Vec<(&str, Option<&str>)> = match provider.as_str() {
                "github" => {
                    let mut list: Vec<(&str, Option<&str>)> = BASE_WORKFLOWS.iter().map(|n| (*n, None)).collect();
                    if opts.deploy { list.push((DEPLOY_WORKFLOW, None)); }
                    if opts.security_scan { list.push((SECURITY_SCAN_WORKFLOW, None)); list.push((DENY_TOML, Some("."))); }
                    if opts.coverage { list.push((COVERAGE_WORKFLOW, None)); }
                    if opts.actionlint { list.push((ACTIONLINT_WORKFLOW, None)); }
                    if opts.healthcheck { list.push((HEALTHCHECK_WORKFLOW, None)); }
                    if matches.get_flag("release") { list.push((RELEASE_WORKFLOW, None)); }
                    if opts.matrix { list.push((MATRIX_WORKFLOW, None)); }
                    if opts.dependabot { list.push((DEPENDABOT_CONFIG, Some(".github"))); }
                    list
                }
                "gitlab" => vec![(".gitlab-ci.yml", None)],
                "bitbucket" => vec![("bitbucket-pipelines.yml", None)],
                "azure" => vec![("azure-pipelines.yml", None)],
                "circleci" => vec![("config.yml", None)],
                "woodpecker" => vec![(".woodpecker.yml", None)],
                "buildkite" => vec![("pipeline.yml", None)],
                _ => return Err(ForgeError::InvalidArgument(format!(
                    "unknown provider `{provider}` (available: {})",
                    available_providers().join(", ")
                ))),
            };

            for (preset_name, dest_rel_override) in selected.drain(..) {
                let file = provider_dir
                    .get_file(format!("{provider}/{preset_name}"))
                    .ok_or_else(|| ForgeError::Template(format!("missing preset file {provider}/{preset_name}")))?;
                let contents = file
                    .contents_utf8()
                    .ok_or_else(|| ForgeError::Template(format!("preset {preset_name} is not UTF-8")))?;
                let dest_rel = dest_rel_override.unwrap_or(output_dir(provider));
                let out_path = dir.join(dest_rel).join(preset_name);
                let vars = {
                    let mut vars = Vars::new();
                    vars.insert("project_name".into(), name.to_string());
                    vars.insert("crate_name".into(), name.replace('-', "_"));
                    vars.insert("msrv".into(), opts.msrv.clone().unwrap_or_else(|| DEFAULT_MSRV.to_string()));
                    vars.insert("max_size".into(), max_size.to_string());
                    vars
                };
                let rendered = render_preset(&dir, provider, contents, &vars)?;
                if out_path.exists() {
                    let existing = std::fs::read_to_string(&out_path)
                        .map_err(ForgeError::io(format!("reading {}", out_path.display())))?;
                    if existing == rendered {
                        continue;
                    }
                    print_diff(&out_path, &rendered)?;
                } else {
                    let parent = out_path.parent().unwrap_or(&dir);
                    std::fs::create_dir_all(parent).map_err(ForgeError::io(format!("creating {}", parent.display())))?;
                    print_diff(&out_path, &rendered)?;
                }
            }
            return Ok(());
        }

        let written = generate(
            &dir,
            provider,
            &name,
            opts.deploy,
            matches.get_flag("release"),
            &opts,
            matches.get_flag("force"),
        )?;

        if ctx.json {
            let report = serde_json::json!({
                "provider": provider,
                "project_name": name,
                "written_files": written,
            });
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
        } else if !ctx.quiet {
            print!(
                "{}",
                format_report(
                    provider,
                    &name,
                    &written,
                    matches.get_flag("release"),
                    &opts
                )
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_opts() -> GenerateOptions {
        GenerateOptions::default()
    }

    #[allow(dead_code)]
    fn deploy_opts() -> GenerateOptions {
        GenerateOptions {
            deploy: true,
            ..Default::default()
        }
    }

    #[test]
    fn coverage_defaults_to_false() {
        assert!(!GenerateOptions::default().coverage);
    }

    #[test]
    fn coverage_flag_is_parsed() {
        let matches = CiPresetsPlugin
            .command()
            .try_get_matches_from(["ci-init", "--coverage"])
            .unwrap();
        assert!(matches.get_flag("coverage"));
    }

    #[test]
    fn actionlint_and_max_size_flags_are_parsed() {
        let matches = CiPresetsPlugin
            .command()
            .try_get_matches_from(["ci-init", "--actionlint", "--max-size", "65536"])
            .unwrap();
        assert!(matches.get_flag("actionlint"));
        assert_eq!(matches.get_one::<u64>("max-size"), Some(&65_536));
    }

    #[test]
    fn github_preset_generates_coverage_only_when_enabled() {
        let default_dir = tempfile::tempdir().unwrap();
        let default_written = generate(
            default_dir.path(),
            "github",
            "my-contract",
            false,
            false,
            &base_opts(),
            false,
        )
        .unwrap();
        assert_eq!(
            default_written,
            vec![
                ".github/workflows/build-test.yml",
                ".github/workflows/contract-size.yml"
            ]
        );
        assert!(!default_dir
            .path()
            .join(".github/workflows/coverage.yml")
            .exists());

        let coverage_dir = tempfile::tempdir().unwrap();
        let coverage_written = generate(
            coverage_dir.path(),
            "github",
            "my-contract",
            false,
            false,
            &GenerateOptions {
                coverage: true,
                ..Default::default()
            },
            false,
        )
        .unwrap();
        assert!(coverage_written
            .iter()
            .any(|p| p == ".github/workflows/coverage.yml"));
        let contents =
            std::fs::read_to_string(coverage_dir.path().join(".github/workflows/coverage.yml"))
                .unwrap();
        assert!(contents.contains("codecov/codecov-action@v4"));
        assert!(contents.contains("cargo llvm-cov"));
    }

    #[test]
    fn github_step_override_is_applied_again_when_force_regenerating() {
        let dir = tempfile::tempdir().unwrap();
        let override_path = dir.path().join(
            ".soroban-forge/ci-overrides/github/build-test/build-and-test/rust-cache.yml",
        );
        std::fs::create_dir_all(override_path.parent().unwrap()).unwrap();
        std::fs::write(
            &override_path,
            "- uses: Swatinem/rust-cache@v2\n  with:\n    key: custom-${{ runner.os }}-${{ hashFiles('Cargo.lock') }}\n",
        )
        .unwrap();

        let opts = base_opts();
        generate(
            dir.path(),
            "github",
            "my-contract",
            false,
            false,
            &opts,
            false,
        )
        .unwrap();

        let workflow_path = dir.path().join(".github/workflows/build-test.yml");
        let generated = std::fs::read_to_string(&workflow_path).unwrap();
        assert!(
            generated.contains("key: custom-${{ runner.os }}-${{ hashFiles('Cargo.lock') }}")
        );
        assert!(!generated.contains("ci-init-step:"));

        std::fs::write(&workflow_path, "stale generated workflow\n").unwrap();
        generate(
            dir.path(),
            "github",
            "my-contract",
            false,
            false,
            &opts,
            true,
        )
        .unwrap();

        let regenerated = std::fs::read_to_string(workflow_path).unwrap();
        assert!(
            regenerated.contains("key: custom-${{ runner.os }}-${{ hashFiles('Cargo.lock') }}")
        );
        assert!(!regenerated.contains("stale generated workflow"));
    }

    #[test]
    fn stale_flag_is_parsed() {
        let matches = CiPresetsPlugin
            .command()
            .try_get_matches_from(["ci-init", "--stale"])
            .unwrap();
        assert!(matches.get_flag("stale"));
    }

    #[test]
    fn available_providers_includes_github_gitlab_circleci_azure() {
        let providers = available_providers();
        assert!(providers.contains(&"github"));
        assert!(providers.contains(&"gitlab"));
        assert!(providers.contains(&"circleci"));
        assert!(providers.contains(&"bitbucket"));
        assert!(providers.contains(&"azure"));
        assert!(providers.contains(&"woodpecker"));
        assert!(providers.contains(&"buildkite"));
    }

    #[test]
    fn writes_azure_preset() {
        let dir = tempfile::tempdir().unwrap();
        let written = generate(
            dir.path(),
            "azure",
            "my-contract",
            false,
            false,
            &base_opts(),
            false,
        )
        .unwrap();
        assert_eq!(written, vec!["azure-pipelines.yml"]);
        let contents = std::fs::read_to_string(dir.path().join("azure-pipelines.yml")).unwrap();
        assert!(contents.contains("CI/CD configuration for my-contract"));
        assert!(contents.contains("cargo test"));
        assert!(contents.contains("cargo build --target wasm32v1-none --release"));
        assert!(!contents.contains("{{project_name}}"));
    }

    #[test]
    fn writes_bitbucket_preset() {
        let dir = tempfile::tempdir().unwrap();
        let written = generate(
            dir.path(),
            "bitbucket",
            "my-contract",
            false,
            false,
            &base_opts(),
            false,
        )
        .unwrap();
        assert_eq!(written, vec!["bitbucket-pipelines.yml"]);
    }

    #[test]
    fn writes_woodpecker_preset() {
        let dir = tempfile::tempdir().unwrap();
        let written = generate(
            dir.path(),
            "woodpecker",
            "my-contract",
            false,
            false,
            &base_opts(),
            false,
        )
        .unwrap();
        assert_eq!(written, vec![".woodpecker.yml"]);
        let contents = std::fs::read_to_string(dir.path().join(".woodpecker.yml")).unwrap();
        assert!(contents.contains("my-contract"));
        assert!(contents.contains("cargo test"));
        assert!(contents.contains("cargo build --target wasm32v1-none --release"));
        assert!(contents.contains("cargo clippy --all-targets -- -D warnings"));
        assert!(!contents.contains("{{project_name}}"));
    }

    #[test]
    fn writes_buildkite_preset() {
        let dir = tempfile::tempdir().unwrap();
        let written = generate(
            dir.path(),
            "buildkite",
            "my-contract",
            false,
            false,
            &base_opts(),
            false,
        )
        .unwrap();
        assert_eq!(written, vec![".buildkite/pipeline.yml"]);
        let contents = std::fs::read_to_string(dir.path().join(".buildkite/pipeline.yml")).unwrap();
        assert!(contents.contains("my-contract"));
        assert!(contents.contains("cargo test"));
        assert!(contents.contains("cargo build --target wasm32v1-none --release"));
        assert!(contents.contains("MAX_WASM_BYTES"));
        assert!(!contents.contains("{{project_name}}"));
        assert!(!contents.contains("{{max_size}}"));
    }

    /// Structural validation against Buildkite's pipeline schema
    /// (https://buildkite.com/docs/pipelines/configure/step-types/command-step):
    /// a pipeline is a mapping with a `steps` sequence, and every command step
    /// (one with a `command`/`commands` key, as opposed to `wait`/`block`/
    /// `input`/`trigger`/`group`) has the shapes those keys require.
    fn assert_valid_buildkite_pipeline(yaml: &str) {
        let doc: serde_yaml::Value = serde_yaml::from_str(yaml).expect("valid YAML");
        let root = doc.as_mapping().expect("pipeline root must be a mapping");

        if let Some(env) = root.get("env") {
            let env = env.as_mapping().expect("`env` must be a mapping");
            for (key, _) in env {
                assert!(key.as_str().is_some(), "`env` keys must be strings");
            }
        }

        let steps = root
            .get("steps")
            .expect("pipeline must have a top-level `steps` key")
            .as_sequence()
            .expect("`steps` must be a sequence");
        assert!(!steps.is_empty(), "`steps` must not be empty");

        for step in steps {
            let step = step.as_mapping().expect("each step must be a mapping");

            if let Some(label) = step.get("label") {
                assert!(label.as_str().is_some(), "`label` must be a string");
            }
            if let Some(key) = step.get("key") {
                assert!(key.as_str().is_some(), "`key` must be a string");
            }
            if let Some(depends_on) = step.get("depends_on") {
                assert!(
                    depends_on.as_str().is_some() || depends_on.as_sequence().is_some(),
                    "`depends_on` must be a string or a sequence of strings"
                );
            }

            let command = step.get("command").or_else(|| step.get("commands"));
            if let Some(command) = command {
                let is_valid = command.as_str().is_some()
                    || command
                        .as_sequence()
                        .is_some_and(|seq| seq.iter().all(|c| c.as_str().is_some()));
                assert!(
                    is_valid,
                    "`command`/`commands` must be a string or a sequence of strings, got {command:?}"
                );
            } else {
                // Not a command step — one of `wait`/`block`/`input`/`trigger`/`group`,
                // which this preset never emits, but a real pipeline may mix in.
                assert!(
                    step.get("wait").is_some()
                        || step.get("block").is_some()
                        || step.get("input").is_some()
                        || step.get("trigger").is_some()
                        || step.get("group").is_some(),
                    "step has neither `command`/`commands` nor a recognised non-command step key: {step:?}"
                );
                continue;
            }

            if let Some(plugins) = step.get("plugins") {
                let plugins = plugins.as_sequence().expect("`plugins` must be a sequence");
                for plugin in plugins {
                    let plugin = plugin
                        .as_mapping()
                        .expect("each plugin entry must be a mapping");
                    assert_eq!(
                        plugin.len(),
                        1,
                        "each plugin entry must have exactly one key (the plugin reference)"
                    );
                }
            }
        }
    }

    #[test]
    fn buildkite_pipeline_validates_against_schema() {
        let dir = tempfile::tempdir().unwrap();
        generate(
            dir.path(),
            "buildkite",
            "my-contract",
            false,
            false,
            &base_opts(),
            false,
        )
        .unwrap();
        let contents = std::fs::read_to_string(dir.path().join(".buildkite/pipeline.yml")).unwrap();
        assert_valid_buildkite_pipeline(&contents);

        // The steps this preset is required to emit (#240): build, test and
        // contract-size, mirroring the GitHub build-test preset's jobs.
        assert!(contents.contains("build & test"));
        assert!(contents.contains("contract size"));
    }

    #[test]
    fn github_release_preset_is_emitted_when_requested() {
        let dir = tempfile::tempdir().unwrap();
        let written = generate(dir.path(), "github", "my-contract", false, true, &base_opts(), false).unwrap();
        assert!(written.iter().any(|path| path == ".github/workflows/release.yml"));
        let contents = std::fs::read_to_string(dir.path().join(".github/workflows/release.yml")).unwrap();
        assert!(contents.contains("cargo build --target wasm32v1-none --release --locked"));
        assert!(contents.contains("softprops/action-gh-release@v2"));
        assert!(contents.contains("v*.*.*"));
    }

    #[test]
    fn ci_init_path_flag_is_honored() {
        let matches = CiPresetsPlugin
            .command()
            .try_get_matches_from(["ci-init", "--path", "../contracts/demo"])
            .unwrap();
        let dir = matches
            .get_one::<String>("path")
            .map(|path| std::path::PathBuf::from(path))
            .unwrap();
        assert_eq!(dir, std::path::PathBuf::from("../contracts/demo"));
    }
}
