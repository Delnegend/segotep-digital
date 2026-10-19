# Default recipe runs when `just` is invoked without arguments
default:
    @just --list

# Run verification suite (formatting, clippy, tests)
check: fmt-check lint test

# Check code formatting
fmt-check:
    cargo fmt --check

# Format code
fmt:
    cargo fmt

# Run Clippy lints with warnings as errors
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Run all tests
test:
    cargo test --all-targets --all-features

# Bump Cargo manifest version and update lockfile
bump version:
    sed -i -E '0,/^version = ".*"/s//version = "{{version}}"/' Cargo.toml
    cargo check

# Language-agnostic version contract for Delnegend/actions/resolve-release.
# `just version get` prints X.Y.Z; `just version bump X.Y.Z` rewrites manifests.
version action ver="":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{action}}" in
      get)
        python3 -c "import tomllib;d=tomllib.load(open('Cargo.toml','rb'));print(d.get('workspace',{}).get('package',{}).get('version') or d.get('package',{}).get('version',''))"
        ;;
      bump)
        [ -n "{{ver}}" ] || { echo "usage: just version bump X.Y.Z" >&2; exit 1; }
        sed -i -E '0,/^version = ".*"/s//version = "{{ver}}"/' Cargo.toml
        cargo check
        ;;
      *) echo "usage: just version get|bump X.Y.Z" >&2; exit 1 ;;
    esac
