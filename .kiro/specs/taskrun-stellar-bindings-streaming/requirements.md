# Requirements Document: Live Output Streaming for Stellar Bindings Generation

## Introduction

The `soroban-forge bindings ts` command generates TypeScript client bindings from a compiled Stellar smart contract using the underlying `stellar contract bindings typescript` CLI. Currently, all output from this CLI is captured silently and only displayed if the process fails. Users receive no feedback during the binding generation process, which can take a considerable amount of time, leaving them uncertain whether the operation is progressing or has stalled.

This feature adds support for live output streaming under a `--verbose` flag. When enabled, output from the underlying `stellar contract bindings typescript` CLI is streamed to the console in real-time, providing immediate visibility into the generation progress and helping users understand what bindings and dependencies are being created.

## Glossary

- **Stellar CLI**: The official `stellar` command-line tool that provides contract-related functionality, including `stellar contract bindings typescript`
- **Binding Generation**: The process of generating TypeScript client code that mirrors a Stellar smart contract's interface
- **Output Capture**: The current behavior where subprocess output is stored in memory and only shown if the process fails
- **Live Streaming**: Output being displayed to the console in real-time as it is produced by the subprocess
- **Verbose Flag**: A command-line flag (`--verbose`) that enables additional diagnostic or informational output
- **ForgeContext**: The shared context passed to all Forge plugins containing CLI configuration, current working directory, and JSON output flag
- **ForgePlugin**: The trait that Forge commands implement to integrate with the CLI framework

## Requirements

### Requirement 1: Enable Live Streaming Under Verbose Flag

**User Story:** As a developer generating Stellar contract bindings, I want to see real-time progress from the underlying `stellar contract bindings typescript` CLI when I use a `--verbose` flag, so that I can monitor the generation process and understand what is happening.

#### Acceptance Criteria

1. THE `BindingsTsPlugin` command `ts` subcommand SHALL accept a `--verbose` flag
2. WHEN the `--verbose` flag is provided, THE `run_stellar_bindings` function SHALL stream stdout and stderr from the `stellar contract bindings typescript` process to the console in real-time
3. WHEN the `--verbose` flag is not provided, THE `run_stellar_bindings` function SHALL maintain the current behavior of capturing all output and only displaying it on failure
4. WHEN the `--verbose` flag is provided AND the process fails, THE `run_stellar_bindings` function SHALL display both the real-time streamed output AND the captured exit error status

### Requirement 2: Default Behavior Remains Unchanged

**User Story:** As a developer using the bindings command without explicit verbose flags, I want the default behavior to remain silent and clean, so that existing scripts and CI/CD pipelines continue to work as expected.

#### Acceptance Criteria

1. THE `soroban-forge bindings ts` command WITHOUT `--verbose` SHALL produce the same console output as before (no stellar-cli output)
2. THE `soroban-forge bindings ts` command SHALL only show bindings generation output if verbose mode is explicitly requested
3. WHEN the process completes successfully in non-verbose mode, THE command SHALL display the existing success message: "generated TypeScript bindings from {wasm_path} -> {output_dir}"

### Requirement 3: Verbose Flag Propagates Through Watch Mode

**User Story:** As a developer using `--watch` mode with verbose output, I want binding generation progress to be visible on each regeneration, so that I can monitor continuous integration and development cycles.

#### Acceptance Criteria

1. THE `--verbose` flag SHALL be accepted alongside the `--watch` flag
2. WHEN both `--watch` and `--verbose` flags are provided, THE `watch_loop` function SHALL stream output on each regeneration cycle
3. WHEN `--watch` is used without `--verbose`, the watch loop SHALL maintain silent operation (current behavior)
4. WHEN a file change triggers regeneration in watch mode with `--verbose`, THE stellar-cli output SHALL be streamed for each regeneration

### Requirement 4: Proper Error Handling in Verbose Mode

**User Story:** As a developer troubleshooting a failed binding generation, I want to see error output immediately and clearly when verbose mode is enabled, so that I can diagnose issues quickly.

#### Acceptance Criteria

1. WHEN the `stellar contract bindings typescript` process fails in verbose mode, THE actual error status and messages SHALL be visible (already streamed)
2. WHEN a non-zero exit code is received, THE `run_stellar_bindings` function SHALL still return an error with context about the failure
3. WHEN an error occurs in non-verbose mode, the captured stderr output SHALL still be included in the error message as currently implemented

### Requirement 5: Consistent Verbose Behavior with Command Context

**User Story:** As a Forge user expecting consistent verbose behavior across commands, I want the `--verbose` flag to follow Forge's existing verbose patterns, so that the CLI experience is predictable and cohesive.

#### Acceptance Criteria

1. THE `--verbose` flag on the `ts` subcommand SHALL be independent and not confused with any existing `--json` flag behavior (which goes to `ForgeContext`)
2. WHEN `--verbose` and `--json` flags are both provided, THE `--json` flag SHALL control JSON output format and `--verbose` SHALL control subprocess output streaming
3. THE `--verbose` flag implementation SHALL respect the existing `ForgeContext` and maintain compatibility with logging/output infrastructure

### Requirement 6: Documentation and Help Text

**User Story:** As a developer learning about the bindings command, I want to understand what the `--verbose` flag does through help text, so that I can use it effectively.

#### Acceptance Criteria

1. THE `ts` subcommand help text SHALL include clear documentation for the `--verbose` flag
2. THE help text SHALL explain that verbose mode streams output from the underlying stellar-cli tool
3. THE help text for `--verbose` SHALL indicate it is useful for monitoring long-running generation tasks

### Requirement 7: Parser and Serializer Requirements

**User Story:** As a developer ensuring robustness, I want the output streaming implementation to correctly parse and serialize process output, so that no data is lost or corrupted.

#### Acceptance Criteria

1. THE output from `stellar contract bindings typescript` stdout SHALL be streamed to the console without buffering or modification
2. THE output from `stellar contract bindings typescript` stderr SHALL be streamed to the console without buffering or modification  
3. WHEN both stdout and stderr are produced simultaneously, THE streaming implementation SHALL preserve output ordering or display both streams clearly
4. FOR ALL valid stellar-cli process outputs, streaming the output and then capturing exit status SHALL produce equivalent error/success information to the current capture-then-report approach (round-trip property)
