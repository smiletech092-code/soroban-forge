# ADR 0002: Render templates with simple variable substitution

- Status: Accepted
- Date: 2026-09-29

## Context

Scaffolding needs to turn bundled template trees into standalone project
directories while supporting generated names, author metadata, SDK versions,
and optional template-specific values. Templates must remain easy to read and
must also be safe to render when files contain syntax that resembles a
placeholder, such as GitHub Actions expressions.

## Decision

Keep templates as files under `templates/` and embed them in the scaffold
binary. Use the shared core `render_str` function to replace only `{{name}}`
placeholders whose names exist in the supplied variable map. Leave unknown or
unterminated placeholders untouched. Apply substitution to UTF-8 file contents
and relative paths; strip a trailing `.hbs` from rendered file names so
`Cargo.toml.hbs` becomes `Cargo.toml`. Do not copy the root `template.toml`;
use it only to declare metadata, defaults, and optional variables.

Compose common files from `templates/_partials/` when a template has not shipped
its own, and substitute the release-profile marker with the shared Cargo
profile. Generate `forge.toml` alongside the rendered template. For cloned
filesystem templates, preserve binary files without text substitution.

## Consequences

- Templates stay as reviewable source files and are available without runtime
  template downloads.
- The renderer has a small, explicit feature set rather than a full template
  language; control flow and richer transformations belong in Rust code.
- Unknown placeholders survive rendering, which preserves embedded syntaxes
  but means misspelled template variables need tests or review to catch.
- `.hbs` avoids Cargo treating source templates as real packages.
- Partials reduce duplicated shared files while template-provided files can
  override the shared defaults.

## References

- [`render_str`](../../crates/core/src/render.rs)
- [Scaffold template embedding and rendering](../../crates/scaffold/src/lib.rs)
- [Template authoring guide](../templates.md)