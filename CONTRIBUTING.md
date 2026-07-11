# Contributing

## Development checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path tests/renamed-dependency/Cargo.toml
```

The minimum supported Rust version is 1.86.

## Commit messages

This project uses [Conventional Commits](https://www.conventionalcommits.org/). Use a lowercase type, an optional scope, and a concise description:

```text
feat: add counter instrumentation
fix(labels): preserve borrowed values
docs: explain recorder installation
feat!: change generated metric names
```

Common types are `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`,
`ci`, and `chore`. Use `!` or a `BREAKING CHANGE:` footer for incompatible
public API changes.

Enable the repository's local commit-message hook after cloning:

```bash
git config core.hooksPath .githooks
```

CI validates every non-merge commit introduced by a push or pull request, so
the hook is a convenience rather than the enforcement boundary. If pull
requests are squash-merged, their titles must also follow Conventional
Commits.

## Releases and changelog

[release-plz](https://release-plz.dev/) opens release pull requests that update
workspace versions and `CHANGELOG.md` from Conventional Commit messages. The
facade and macro crates are versioned together, with macro-crate changes folded
into the public crate's changelog. Release PR automation starts after the
initial `v0.1.0` tag exists.

The first crates.io release must be published manually, with
`function-metrics-macros` published before `function-metrics`. Bootstrap the
release automation after both packages are available:

```bash
cargo publish -p function-metrics-macros
# Wait until the macros package is visible in the crates.io index.
cargo publish -p function-metrics
git tag -a v0.1.0 -m "v0.1.0"
git push origin v0.1.0
gh release create v0.1.0 --verify-tag --generate-notes
gh workflow run Release-plz
```

After crates.io has both crate names and trusted publishing is configured, the
release workflow can also be extended to run `release-plz release`.

Release PRs currently use the repository's `GITHUB_TOKEN`. GitHub does not
start other workflows for pull requests created by that token. Close and reopen
a generated release PR before merging it so that CI runs, or replace the token
with a narrowly scoped GitHub App or personal access token.
