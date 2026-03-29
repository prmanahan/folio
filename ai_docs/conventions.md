# Code Conventions

- **Naming**: `snake_case` for functions/variables, `CamelCase` for types/structs/enums
- **Error handling**: `Result<T, E>` with descriptive error types; avoid `.unwrap()` in library code
- **Doc comments**: `///` on all public API items
- **CLI**: use `clap` with derive macros for argument parsing
- **Formatting**: always run `cargo fmt` before committing

### Dependency rules

- Workspace-level `[workspace.dependencies]` for shared crate versions.
