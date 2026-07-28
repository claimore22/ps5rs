# Contributing to ps5rs

Thanks for your interest in contributing. AI tools are welcome here — they are tools, and their value depends on the person wielding them. Use whatever helps you produce correct, well-tested code.

## Prerequisites

- **Rust** — edition 2024, MSRV 1.85+ (`rustup update stable`)
- **Just** (optional) — task runner for common commands ([installation](https://github.com/casey/just#installation))

## Getting Started

```sh
git clone https://github.com/claimore22/ps5rs.git
cd ps5rs
cargo build --release
cargo test --workspace
```

## Development Workflow

1. Create a feature branch from `master`:
   ```sh
   git checkout -b feat/my-feature master
   ```
2. Make your changes.
3. Run the full check suite before pushing:
   ```sh
   just check          # or manually:
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
4. Commit with a [conventional commit](https://www.conventionalcommits.org/) message:
   ```
   feat(ps5-analysis): add new report type
   fix(ps5-nid): handle edge case in hash
   docs: update README usage section
   chore: bump MSRV to 1.85
   ```
5. Push and open a pull request against `master`.

## Project Structure

```
crates/
  ps5-format    Shared types, errors, ELF constants
  ps5-self      SELF container parser
  ps5-elf       ELF64 binary format parser
  ps5-nid       NID hash algorithm + name catalog
  ps5-image     BinaryImage IR (normalized abstraction)
  ps5-analysis  Analysis engine: scanner, reports, export
  ps5-dashboard Static HTML dashboard generator
  ps5-cli       Command-line interface
data/
  nids.csv      NID hash-to-name database (~154K entries, not tracked in git)
```

## Coding Conventions

- **Formatting**: `cargo fmt` with defaults (see `rustfmt.toml`)
- **Linting**: `cargo clippy --workspace --all-targets -- -D warnings`
- **Tests**: All new functionality must include tests. Run `cargo test --workspace` to verify.
- **Comments**: Only add comments when they explain *why*, not *what*. The code should be self-documenting.
- **Error handling**: Use `?` or explicit `unwrap_or_else` with descriptive messages. Never silently swallow errors.
- **Safety**: `unsafe_code = "forbid"` at workspace level. All parsing is bounds-checked.

## Testing

```sh
cargo test --workspace                    # all tests
cargo test -p ps5-nid                     # single crate
cargo test -- --test-threads=1            # serial execution (if needed)
```

Tests live in `#[cfg(test)] mod tests` blocks within each source file, and in `tests/` directories for integration tests.

## Submitting Changes

- Keep PRs focused — one logical change per PR.
- Ensure CI passes (fmt, clippy, tests on Linux + Windows).
- Add tests for new functionality.
- Update documentation if changing public APIs or CLI behavior.

## Reporting Issues

Open a GitHub issue with:
- What you expected to happen
- What actually happened
- Steps to reproduce
- Your OS and Rust version (`rustc --version`)

## License

By contributing, you agree that your contributions will be licensed under GPL-2.0-only.
