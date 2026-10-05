# Implementation Tasks: Live Output Streaming for Stellar Bindings Generation

## Task Organization

Tasks are organized by implementation phase:
- **Phase 1**: Core streaming infrastructure and function signatures
- **Phase 2**: Watch mode integration
- **Phase 3**: Testing and validation
- **Phase 4**: Documentation

---

## Phase 1: Core Streaming Infrastructure

### Task 1.1: Add Verbose Flag to Command Definition

**Description**: Add the `--verbose` flag to the `bindings ts` subcommand in `BindingsTsPlugin::command()`.

**Files Modified**: `soroban-forge/crates/binding-ts/src/lib.rs`

**Changes**:
- Add new Arg definition for `--verbose`/`-v` flag with short alias
- Help text: "Stream output from the underlying stellar contract bindings CLI in real-time (useful for monitoring long-running generation)"
- Use `ArgAction::SetTrue`

**Acceptance Criteria**:
- Flag is recognized by clap parser
- `--verbose` and `-v` both work as aliases
- Help text appears in `--help` output
- Flag can be combined with other flags (`--watch`, `--force`, etc.)

**Testing**: Unit test parsing the command with `--verbose` flag

---

### Task 1.2: Extract and Propagate Verbose Flag in run_ts

**Description**: Extract the `--verbose` flag value in `run_ts()` and pass it to downstream functions.

**Files Modified**: `soroban-forge/crates/binding-ts/src/lib.rs`

**Changes**:
- In `run_ts()`: Add `let verbose = matches.get_flag("verbose");`
- Pass `verbose` parameter to `generate_bindings_with_options()`
- Pass `verbose` parameter to `watch_loop()` when watch mode is enabled

**Acceptance Criteria**:
- Flag value is extracted without panicking
- Value propagates to both watch and non-watch code paths
- Existing behavior unchanged when flag is absent (defaults to false)

**Testing**: Unit test verifying flag extraction and no runtime errors

---

### Task 1.3: Update Function Signatures for Verbose Parameter

**Description**: Update `generate_bindings_with_options()` and related functions to accept and propagate the verbose flag.

**Files Modified**: `soroban-forge/crates/binding-ts/src/lib.rs`

**Changes**:
- Update `generate_bindings_with_options()` signature: add `verbose: bool` parameter
- Update `run_stellar_bindings()` signature: add `verbose: bool` parameter
- Maintain backward compatibility wrapper: keep old `generate_bindings()` calling new function with `verbose=false`
- Pass verbose flag from `generate_bindings_with_options()` to `run_stellar_bindings()`
- Update all call sites to pass the verbose flag

**Acceptance Criteria**:
- Functions accept verbose parameter
- Backward compatible wrapper maintains public API
- All call sites updated without compilation errors
- Default behavior (verbose=false) identical to before

**Testing**: 
- Compilation succeeds
- Existing tests pass with verbose=false
- Unit test verifying parameter threading through call stack

---

### Task 1.4: Implement Conditional Subprocess Spawning in run_stellar_bindings

**Description**: Implement the core streaming logic by conditionally using `spawn()` for verbose mode and `output()` for silent mode.

**Files Modified**: `soroban-forge/crates/binding-ts/src/lib.rs`

**Changes**:
- Refactor `run_stellar_bindings()` to check `verbose` parameter
- **Verbose mode branch**:
  - Use `Command::spawn()` to start process with I/O passthrough
  - Call `child.wait()` to wait for completion
  - Check exit status with `status.success()`
  - If failed: return error with exit code
- **Silent mode branch** (existing logic):
  - Keep current `.output()` implementation unchanged
  - Capture stderr and include in error messages on failure

**Acceptance Criteria**:
- When `verbose=true`: I/O passes through to console, subprocess output visible to user
- When `verbose=false`: I/O is captured, output only shown on error (current behavior)
- Error handling works correctly in both modes
- Missing stellar-cli error is still properly detected and reported

**Testing**:
- Integration test with real stellar-cli call (or mock subprocess)
- Verify real-time output appears in verbose mode
- Verify silent operation in non-verbose mode
- Test error cases in both modes

---

## Phase 2: Watch Mode Integration

### Task 2.1: Update watch_loop Signature and Calls

**Description**: Add verbose parameter to `watch_loop()` and ensure it's passed through on each regeneration.

**Files Modified**: `soroban-forge/crates/binding-ts/src/lib.rs`

**Changes**:
- Update `watch_loop()` signature: add `verbose: bool` parameter
- In the file watching loop, pass `verbose` to `generate_bindings_with_options()` calls
- Ensure each regeneration respects the verbose flag

**Acceptance Criteria**:
- Watch mode accepts and respects `--verbose` flag
- Streaming occurs on each file change when verbose is true
- Silent operation on each file change when verbose is false
- Watch mode continues to work without verbose flag

**Testing**:
- Integration test: `--watch --verbose` and trigger file changes, verify output streams each time
- Integration test: `--watch` (no verbose) and verify silent operation

---

### Task 2.2: Verify Watch Mode Output Behavior

**Description**: Validate that watch mode produces appropriate output and doesn't break with streaming changes.

**Files Modified**: `soroban-forge/crates/binding-ts/src/lib.rs` (if needed for status messages)

**Changes**:
- May need to adjust status messages when operating in verbose mode within watch loop
- Ensure "regenerated..." messages are still shown between cycles

**Acceptance Criteria**:
- Users see progress messages between watch regeneration cycles
- Output is clear and organized
- No mixing of messages with subprocess output

**Testing**: Manual testing and integration tests

---

## Phase 3: Testing and Validation

### Task 3.1: Write Unit Tests for Flag Parsing and Function Threading

**Description**: Add unit tests verifying the verbose flag is correctly parsed and threaded through functions.

**Files Modified**: `soroban-forge/crates/binding-ts/src/lib.rs` (tests module)

**Changes**:
- Test that `BindingsTsPlugin.command()` exposes `--verbose` flag
- Test that `-v` short flag works
- Test that `run_ts()` correctly extracts the verbose flag
- Test that verbose flag can coexist with other flags

**Acceptance Criteria**:
- All parsing tests pass
- No panics or unwraps on valid input
- Flag correctly appears in help text

**Testing**: Run `cargo test` in binding-ts crate

---

### Task 3.2: Write Integration Tests for Silent Mode (Backward Compatibility)

**Description**: Add integration tests verifying that non-verbose mode produces identical behavior to the original implementation.

**Files Modified**: `soroban-forge/crates/binding-ts/src/lib.rs` (tests module)

**Changes**:
- Test binding generation without `--verbose` flag
- Verify output contains only success message, not stellar-cli details
- Verify bindings are correctly generated
- Compare output and results to baseline

**Acceptance Criteria**:
- Silent mode output identical to pre-feature behavior
- Bindings generated correctly
- Error handling unchanged for non-verbose case

**Testing**: Integration test with real or mocked stellar-cli

---

### Task 3.3: Write Integration Tests for Verbose Streaming Mode

**Description**: Add integration tests verifying that verbose mode correctly streams subprocess output.

**Files Modified**: `soroban-forge/crates/binding-ts/src/lib.rs` (tests module)

**Changes**:
- Create mock or use real stellar-cli invocation
- Run with `--verbose` flag
- Capture console output and verify stellar-cli output is present
- Verify real-time nature of output

**Acceptance Criteria**:
- Stellar-cli output visible when verbose mode is used
- Output appears on stdout/stderr as expected
- Errors are properly reported with context
- Process exit status is correctly handled

**Testing**: Integration test with controlled subprocess

---

### Task 3.4: Write Integration Tests for Watch Mode with Verbose

**Description**: Add integration tests verifying watch mode respects the verbose flag.

**Files Modified**: `soroban-forge/crates/binding-ts/src/lib.rs` (tests module)

**Changes**:
- Test `--watch --verbose` combination
- Trigger file changes and verify streaming occurs on each cycle
- Test `--watch` without verbose and verify silent operation
- Verify graceful termination of watch loop

**Acceptance Criteria**:
- Watch mode correctly streams in verbose mode
- Watch mode stays silent in non-verbose mode
- Multiple regeneration cycles all respect the flag

**Testing**: Integration test with watch loop

---

### Task 3.5: [PBT] Property-Based Test for Output Streaming Correctness

**Description**: Write a property-based test verifying that streaming output and checking exit status produces equivalent error/success information to capture-then-report.

**Files Modified**: `soroban-forge/crates/binding-ts/src/lib.rs` (tests module) and/or new test file

**Property**: FOR ALL processes that would complete successfully in silent mode, executing them with streaming enabled SHALL produce error/success information equivalent to the capture approach.

**Test Implementation**:
- Use property-based testing framework (proptest or quickcheck)
- Generate various command outputs (stdout, stderr, exit codes)
- Compare behavior of streaming vs. capturing approaches
- Verify no data loss or reordering
- Test edge cases: empty output, very large output, interleaved stdout/stderr

**Acceptance Criteria**:
- Property passes for all generated inputs
- No counterexamples found showing information loss
- Test covers realistic stellar-cli output patterns

**Testing**: Run PBT suite and verify properties hold

---

### Task 3.6: Test Error Handling in Both Modes

**Description**: Add tests for error scenarios to ensure both verbose and non-verbose modes handle failures correctly.

**Files Modified**: `soroban-forge/crates/binding-ts/src/lib.rs` (tests module)

**Changes**:
- Test missing stellar-cli error in both modes
- Test invalid wasm file error in both modes
- Test non-zero exit code handling in both modes
- Verify error messages are informative

**Acceptance Criteria**:
- All error cases properly handled
- Error messages are clear and useful
- Errors reported correctly in both verbose and non-verbose modes

**Testing**: Unit and integration tests for error scenarios

---

## Phase 4: Documentation

### Task 4.1: Update Command Help Text

**Description**: Ensure help text clearly describes the `--verbose` flag and its behavior.

**Files Modified**: `soroban-forge/crates/binding-ts/src/lib.rs` (help string in Arg definition)

**Changes**:
- Verify help text for `--verbose` is clear: "Stream output from the underlying stellar contract bindings CLI in real-time (useful for monitoring long-running generation)"
- Ensure help text is visible in `soroban-forge bindings ts --help`

**Acceptance Criteria**:
- Help text is visible and accurate
- Users understand what the flag does
- Help mentions it's useful for monitoring progress

**Testing**: Manual verification of help output

---

### Task 4.2: Update CLI Reference Documentation

**Description**: Add documentation for the `--verbose` flag in the CLI reference guide.

**Files Modified**: `soroban-forge/docs/cli-reference.md` (or equivalent)

**Changes**:
- Add `--verbose` flag to the `bindings ts` command documentation
- Explain use case: monitoring long-running binding generation
- Provide examples of usage: `soroban-forge bindings ts --verbose`
- Explain interaction with `--watch` mode

**Acceptance Criteria**:
- Documentation is complete and accurate
- Examples are clear
- Users can understand the feature from documentation

**Testing**: Documentation review and manual verification

---

### Task 4.3: Add Changelog Entry

**Description**: Document the new feature in the project's changelog.

**Files Modified**: `CHANGELOG.md` (or equivalent)

**Changes**:
- Add entry in appropriate section (Features)
- Describe: "Added `--verbose` flag to `bindings ts` command to stream stellar contract bindings CLI output in real-time"
- Link to related documentation if applicable

**Acceptance Criteria**:
- Changelog entry is present and clear
- Describes the feature accurately
- Follows project's changelog conventions

**Testing**: Manual review of changelog

---

## Acceptance Test Scenarios

### Scenario 1: Silent Mode (Default)
```bash
$ soroban-forge bindings ts --wasm contract.wasm --out-dir ./bindings
generated TypeScript bindings from contract.wasm
  -> ./bindings
```
No stellar-cli output visible.

### Scenario 2: Verbose Mode
```bash
$ soroban-forge bindings ts --wasm contract.wasm --out-dir ./bindings --verbose
[stellar-cli output streams in real-time...]
generated TypeScript bindings from contract.wasm
  -> ./bindings
```
Stellar-cli output visible during generation.

### Scenario 3: Watch Mode with Verbose
```bash
$ soroban-forge bindings ts --watch --verbose
Watching for changes...
[file changes, regeneration starts]
[stellar-cli output streams in real-time...]
Regenerated bindings
[continues watching]
```
Output streams on each regeneration cycle.

### Scenario 4: Error with Verbose
```bash
$ soroban-forge bindings ts --wasm missing.wasm --verbose
Error: stellar contract bindings typescript failed with exit code: 1
[or more specific error message]
```
Error is clear and stellar-cli output was visible during attempt.

### Scenario 5: Error without Verbose
```bash
$ soroban-forge bindings ts --wasm missing.wasm
Error: stellar contract bindings typescript failed:
[captured stderr output from stellar-cli]
```
Error includes captured output even without verbose mode.

---

## Definition of Done

For this feature to be complete:

- [ ] All Phase 1-4 tasks completed
- [ ] All unit tests passing
- [ ] All integration tests passing
- [ ] PBT passes without counterexamples
- [ ] Backward compatibility verified (existing tests pass with `verbose=false`)
- [ ] Manual testing confirms streaming works as expected
- [ ] Documentation updated
- [ ] Code reviewed and approved
- [ ] Feature branch ready for merge to main
