# Installation directories (overridable)
cargo_bin := env_var_or_default("CARGO_HOME", env_var("HOME") / ".cargo") / "bin"
comp_dir := env_var("HOME") / ".local/share/zsh/site-functions"

# List available recipes
default:
    @just --list

# Build debug binary
build:
    cargo build

# Build release binary
release:
    cargo build --release

# Check compilation without building
check:
    cargo check

# Run clippy lints
clippy:
    cargo clippy

# Format source code
fmt:
    cargo fmt

# Check formatting and run clippy
lint:
    cargo fmt --check
    cargo clippy

# Run all tests (Rust unit tests and BDD tests)
test:
    cargo test
    behave

# Install release binary, create tk symlink, and install completions
install: release
    cargo install --path .
    ln -sf "{{cargo_bin}}/ticket" "{{cargo_bin}}/tk"
    mkdir -p "{{comp_dir}}"
    cp completions/_tk "{{comp_dir}}/_tk"
    @echo "Installed: {{cargo_bin}}/ticket"
    @echo "Symlinked: {{cargo_bin}}/tk -> ticket"
    @echo "Completions: {{comp_dir}}/_tk"

# Uninstall binary, symlink, and completions
uninstall:
    cargo uninstall ticket-rs
    rm -f "{{cargo_bin}}/tk"
    rm -f "{{comp_dir}}/_tk"
    @echo "Uninstalled ticket-rs"

# Remove build artifacts
clean:
    cargo clean
