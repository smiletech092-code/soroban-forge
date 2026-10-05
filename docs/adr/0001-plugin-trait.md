# ADR 0001: Use a trait-based plugin interface

- Status: Accepted
- Date: 2026-09-29

## Context

Built-in commands should share one dispatch path while remaining independently
implemented in small crates. Third-party commands should also be possible
without compiling them into the main binary. Plugins need access to invocation
state and a consistent error/result contract.

## Decision

Define `ForgePlugin` in `soroban-forge-core`. Each built-in command implements
the trait by providing its name, a `clap::Command`, and a `run` method that
receives the parsed arguments and a shared `ForgeContext`, returning the core
`Result`. Optional pre- and post-run hooks use the same context and error type.
The binary registers built-in plugins as `Vec<Box<dyn ForgePlugin>>`; the core
owns command assembly, argument parsing, context construction, and dispatch.

External extensions use the `soroban-forge-<name>` executable convention.
When no built-in matches, the CLI looks for that executable on `PATH`, forwards
arguments and supported global flags, and propagates its exit status.

## Consequences

- Command implementations can live in separate crates and depend on the core
  interface rather than one another.
- Shared context and error handling make built-ins consistent and testable.
- External plugins need not link into the CLI, but process execution gives the
  host less control over their behavior than built-in plugins.
- Adding a built-in requires registering it in the binary; external commands
  are discovered by naming convention.

## References

- [`ForgePlugin` and `ForgeContext`](../../crates/core/src/plugin.rs)
- [Built-in plugin registration](../../src/main.rs)
- [CLI dispatch](../../crates/core/src/cli.rs)
- [Plugin authoring guide](../plugin-tutorial.md)