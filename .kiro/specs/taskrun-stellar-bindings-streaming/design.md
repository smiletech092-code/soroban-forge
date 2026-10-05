# Design Document: Live Output Streaming for Stellar Bindings Generation

## Overview

This document describes the technical design for adding live output streaming to the `soroban-forge bindings ts` command when the `--verbose` flag is used. The implementation allows users to monitor the underlying `stellar contract bindings typescript` CLI's progress in real-time while maintaining complete backward compatibility with the current silent default behavior.

## Acceptance Criteria Testing Strategy

### Criteria 1.1: Verbose Flag Accepted
**Testing Approach:** Example-based unit test
- Parse command line with `--verbose` flag and verify it's captured in ArgMatches

### Criteria 1.2: Live Streaming When Verbose
**Testing Approach:** Integration test with mocked subprocess
- Capture actual streamed output and verify it appears on console during execution
- Use a test subprocess that produces output over a short duration
- Verify output appears before process completes

### Criteria 1.3: Silent Mode When Not Verbose
**Testing Approach:** Integration test
- Run binding generation without verbose flag
- Verify no stellar-cli output appears on stdout/stderr regardless of process output
- Verify existing success message still appears

### Criteria 1.4: Streaming + Error Status
**Testing Approach:** Integration test with failing subprocess
- Run with `--verbose` against a failing stellar-cli invocation
- Verify output was streamed AND error is properly returned with context

### Criteria 2.1-2.3: Default Behavior
**Testing Approach:** Integration tests
- Verify output format and messages match baseline (existing behavior)
- Verify bindings are generated identically in both modes

### Criteria 3.1-3.4: Watch Mode Integration
**Testing Approach:** Integration test with watch loop
- Run with `--watch --verbose` and trigger file changes
- Verify streaming occurs on each cycle
- Run with `--watch` (no verbose) and verify silent operation

### Criteria 4.1-4.3: Error Handling
**Testing Approach:** Integration tests
- Test various failure scenarios (missing stellar-cli, invalid wasm, etc.)
- Verify errors are properly reported in both modes

### Criteria 5.1-5.3: Context Consistency
**Testing Approach:** Unit and integration tests
- Verify flags don't conflict
- Verify JSON output works independently
- Verify ForgeContext is respected

### Criteria 6.1-6.3: Documentation
**Testing Approach:** Help text validation
- Parse help output and verify `--verbose` is documented
- Verify help text describes the feature clearly

### Criteria 7.1-7.4: Output Streaming Correctness (Round-Trip Property)
**Testing Approach:** Property-based test
- **Property**: FOR ALL processes that would succeed silently, streaming them and then reporting exit status SHOULD produce error/success information equivalent to capturing-then-reporting
- **Counterexample to test**: Find cases where streaming loses information (e.g., interleaved output, buffering issues)
- **Test implementation**: Use hypothesis/proptest to generate various stellar-cli output patterns and verify streaming preserves all data and ordering
- **Mutation testing**: Verify that removing streaming would lose information or change behavior observably

## Architecture

### High-Level Design

The streaming functionality integrates at the point where the subprocess is spawned. The current implementation uses `std::process::Command::new()` with `.output()` which captures all I/O. The new design conditionally uses `.spawn()` with `.wait()` instead when verbose mode is enabled, allowing real-time I/O passthrough.

### Key Components

#### 1. Command-Line Argument Addition

**File**: `soroban-forge/crates/binding-ts/src/lib.rs`

In the `BindingsTsPlugin::command()` method, add a new `--verbose` flag:

```rust
.arg(
    Arg::new("verbose")
        .long("verbose")
        .short('v')
        .action(ArgAction::SetTrue)
        .help("Stream output from the underlying stellar contract bindings CLI in real-time (useful for monitoring long-running generation)"),
)
```

This flag is added to the `ts` subcommand definition.

#### 2. Flag Propagation Through `run_ts`

In the `run_ts` function, extract the verbose flag:

```rust
let verbose = matches.get_flag("verbose");
```

Pass it down to the binding generation functions:

```rust
let wasm_path = generate_bindings_with_options_verbose(
    &dir,
    wasm_override.as_deref(),
    &output,
    package_name,
    react,
    force,
    verbose,  // NEW PARAMETER
)?;
```

And in the watch loop case:

```rust
if watch {
    return watch_loop(
        &dir,
        wasm_override.as_deref(),
        &output,
        package_name,
        react,
        verbose,  // NEW PARAMETER
        ctx,
    );
}
```

#### 3. Function Signature Updates

Create new function variants that accept the verbose flag:

**Option A (Preferred): Add verbose parameter to existing functions**

Update `generate_bindings_with_options` to add a `verbose: bool` parameter:

```rust
pub fn generate_bindings_with_options(
    contract_dir: &Path,
    wasm_override: Option<&Path>,
    output: &Path,
    package_name: Option<&str>,
    react: bool,
    force: bool,
    verbose: bool,  // NEW
) -> Result<PathBuf>
```

Keep a non-verbose wrapper for backward compatibility:

```rust
pub fn generate_bindings(
    contract_dir: &Path,
    wasm_override: Option<&Path>,
    output: &Path,
    force: bool,
) -> Result<PathBuf> {
    generate_bindings_with_options(contract_dir, wasm_override, output, None, false, force, false)
}
```

Pass verbose to `run_stellar_bindings`:

```rust
run_stellar_bindings(&wasm_path, output, verbose)?;
```

#### 4. Core Streaming Implementation in `run_stellar_bindings`

Update the `run_stellar_bindings` function signature:

```rust
fn run_stellar_bindings(wasm: &Path, output: &Path, verbose: bool) -> Result<()>
```

**Conditional process spawning**:

```rust
if verbose {
    // Real-time streaming mode
    let mut child = std::process::Command::new("stellar")
        .args([...])
        .spawn()
        .map_err(/* error handling */)?;
    
    let status = child.wait()
        .map_err(/* error handling */)?;
    
    if !status.success() {
        return Err(ForgeError::Other(format!(
            "stellar contract bindings typescript failed with exit code: {}",
            status.code().unwrap_or(-1)
        )));
    }
    Ok(())
} else {
    // Current capture behavior (unchanged)
    let result = std::process::Command::new("stellar")
        .args([...])
        .output();
    
    match result {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            Err(ForgeError::Other(format!(
                "stellar contract bindings typescript failed:\n{stderr}"
            )))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(ForgeError::ToolMissing("stellar-cli".into()))
        }
        Err(e) => Err(ForgeError::io(
            "running stellar contract bindings typescript",
        )(e)),
    }
}
```

#### 5. Watch Loop Integration

Update `watch_loop` signature to accept verbose flag and pass it through:

```rust
fn watch_loop(
    contract_dir: &Path,
    wasm_override: Option<&Path>,
    output: &Path,
    package_name: Option<&str>,
    react: bool,
    verbose: bool,  // NEW
    ctx: &ForgeContext,
) -> Result<()>
```

On each regeneration cycle:

```rust
generate_bindings_with_options(
    contract_dir,
    wasm_override,
    output,
    package_name,
    react,
    true,  // force=true in watch loop
    verbose,  // Pass through verbose flag
)?;
```

### Data Flow Diagram

```
User Input
    ↓
[--verbose flag present?]
    ├─ YES → run_ts() verbose=true
    │         ↓
    │    generate_bindings_with_options(..., verbose=true)
    │         ↓
    │    run_stellar_bindings(..., verbose=true)
    │         ↓
    │    Command::spawn() → real-time passthrough I/O
    │         ↓
    │    child.wait() → check exit status
    │         ↓
    │    On error: return error with exit code
    │
    └─ NO → run_ts() verbose=false
            ↓
            generate_bindings_with_options(..., verbose=false)
            ↓
            run_stellar_bindings(..., verbose=false)
            ↓
            Command::output() → capture all I/O
            ↓
            On error: return error with captured stderr
```

## Implementation Considerations

### I/O Handling

When using `spawn()` instead of `output()`, stdout and stderr pass through to the parent process's console by default. The Stellar CLI will write directly to the user's terminal, providing true real-time feedback. This is the intended and simplest approach.

### Error Context in Verbose Mode

In verbose mode, users see the output as it streams, so they get immediate visual feedback. However, we should still wrap the error with context:

```rust
if !status.success() {
    // The user already saw the output stream, so we just need to report failure
    Err(ForgeError::Other(format!(
        "stellar contract bindings typescript failed with exit code: {}",
        status.code().unwrap_or(-1)
    )))
}
```

In non-verbose mode, we preserve the current detailed error including captured stderr.

### Backward Compatibility

- The public API `generate_bindings()` is unchanged and calls the new function with `verbose=false`
- Default behavior (non-verbose) is identical to current implementation
- Existing tests continue to pass with `verbose=false`
- The `--verbose` flag is purely opt-in

### Testing Strategy

1. **Unit tests** for flag parsing and function signatures
2. **Integration tests** with mocked/real stellar-cli calls
3. **Property-based test** for streaming correctness round-trip property
4. **Watch mode tests** verifying streaming behavior on regeneration

## Security Considerations

- No security risk from streaming output; stellar-cli's output is intended for user consumption
- No new file operations or privilege escalations
- Passing through subprocess I/O is standard practice
- Verbose mode doesn't expose any additional information beyond what the CLI already outputs

## Performance Implications

- Minimal impact: verbose mode uses slightly fewer allocations (no capture buffer) but likely negligible
- Non-verbose mode is unchanged
- I/O passthrough is more efficient than buffering and reprinting

## Rollout Strategy

1. Implement feature with feature flag or within normal development cycle
2. Add `--verbose` flag and basic streaming in `run_stellar_bindings`
3. Add watch mode support
4. Add comprehensive tests including property-based tests
5. Update documentation and help text
6. Release as part of normal version (no special rollout needed)
