# Contributing to bnb

Thank you for helping improve `bnb`. We welcome bug reports, feature suggestions, and pull requests.

## Development Setup

To build and test `bnb` locally, you need a working Rust toolchain.

```bash
git clone [https://github.com/EatSleepCode-Repeat/bnb.git](https://github.com/EatSleepCode-Repeat/bnb.git)
cd bnb
cargo build
cargo run
```

## Compiler Requirements & MSRV

Edition: 2021
MSRV (Minimum Supported Rust Version): 1.74.0

We intentionally maintain compatibility with Rust 1.74 to support older stable environments and LTS Linux distributions. PRs introducing dependencies or syntax requiring newer compiler versions should discuss this in an issue first.

## Checks & Testing
Before submitting a pull request, verify that all tests and lints pass locally:

```bash
# Run unit and integration tests
cargo test

# Check code formatting
cargo fmt --check

# Run Clippy (deny warnings)
cargo clippy --all-targets -- -D warnings
```

If you add new syntax rules, built-in commands, or secret detection regexes, please include corresponding unit tests inside the relevant module or under tests/.

## Pull Request Guidelines

- Branch naming: Use descriptive branch names like fix/job-reaping or feat/docker-prompt.
- Commit style: Keep commit messages concise, imperative, and focused (e.g., guard: handle bearer tokens in multiline input).
- Scope: Keep pull requests focused on a single logical change or bug fix.

## Security & Secret Guard Disclosures

If you discover an issue related to credential leakage, vault decryption bypass, or improper process argument exposure in Secret Guard, please do not open a public issue.
Instead, report it via a Private Security Advisory on GitHub (Security > Advisories > New draft security advisory) so it can be patched before public disclosure.
