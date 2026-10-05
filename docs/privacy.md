# Privacy and Telemetry

`soroban-forge` collects no telemetry. It does not send usage analytics, crash
reports, command arguments, project contents, identifiers, or any other data to
the maintainers or to an analytics service.

Some commands make network requests only to perform an operation the user
explicitly requested, such as cloning a remote template, checking testnet RPC
connectivity, funding an identity with friendbot, or fetching deployed contract
Wasm for verification. These requests are functional, not telemetry. Pass
`--offline` to disable all such network access.

### Release-version check

On each invocation (throttled to once per 24 hours), `soroban-forge` contacts
the GitHub releases API at
`https://api.github.com/repos/soroban-forge-labs/soroban-forge/releases/latest`
to check whether a newer version has been published. The only data sent is the
`User-Agent` header (which includes the current version number, e.g.
`soroban-forge/0.1.0`). No user data, command arguments, project contents, or
identifiers are transmitted.

To disable this check:

- Per session: pass `--offline` or set `SOROBAN_FORGE_NO_UPDATE_CHECK=1`.
- Permanently: add `update_check = false` under `[defaults]` in `forge.toml`.

If telemetry is ever introduced, it will be strictly opt-in and disabled by
default. The project will document what is collected, its purpose, destination,
retention period, and how to revoke consent before asking users to enable it.
An upgrade will never silently enroll an existing installation.
