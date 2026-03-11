# Installation directory (overridable)
cargo_bin := env_var_or_default("CARGO_HOME", env_var("HOME") / ".cargo") / "bin"

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

# Install release binary and create tk symlink
install: release
    cargo install --path .
    ln -sf "{{cargo_bin}}/ticket" "{{cargo_bin}}/tk"
    @echo "Installed: {{cargo_bin}}/ticket"
    @echo "Symlinked: {{cargo_bin}}/tk -> ticket"
    @just _prompt-completions

# Offer to install shell completions for the current shell
_prompt-completions:
    #!/usr/bin/env sh
    shell_name=$(basename "${SHELL:-}")
    printf 'Install shell completions for %s? [y/N] ' "$shell_name"
    read -r answer
    case "$answer" in
        [yY]*)
            case "$shell_name" in
                zsh)
                    rc="${ZDOTDIR:-$HOME}/.zshrc"
                    # Source the generated script (binds `ticket`) then alias `tk` to the
                    # same completion function so both command names get completions.
                    printf '\n# ticket shell completions\nsource <(COMPLETE=zsh tk)\ncompdef _clap_dynamic_completer_ticket tk\n' >> "$rc"
                    echo "Added completions to $rc"
                    echo "Restart your shell or run: source $rc"
                    ;;
                bash)
                    rc="$HOME/.bashrc"
                    # Source the generated script (binds `ticket`) then bind `tk` to the
                    # same completion function.
                    printf '\n# ticket shell completions\nsource <(COMPLETE=bash tk)\ncomplete -o nospace -o bashdefault -F _clap_complete_ticket tk\n' >> "$rc"
                    echo "Added completions to $rc"
                    echo "Restart your shell or run: source $rc"
                    ;;
                fish)
                    rc="${XDG_CONFIG_HOME:-$HOME/.config}/fish/config.fish"
                    # Fish uses per-command complete calls; source ticket completions and
                    # add a second binding for tk.
                    printf '\n# ticket shell completions\nCOMPLETE=fish tk | source\ncomplete --keep-order --exclusive --command tk --arguments "(COMPLETE=fish tk -- (commandline --current-process --tokenize --cut-at-cursor) (commandline --current-token))"\n' >> "$rc"
                    echo "Added completions to $rc"
                    echo "Restart your shell or run: source $rc"
                    ;;
                *)
                    echo "Shell '$shell_name' not recognized. Add completions manually:"
                    echo "  See the Shell Completions section in README.md"
                    ;;
            esac
            ;;
        *)
            echo "Skipped. You can add completions manually later:"
            echo "  See the Shell Completions section in README.md"
            ;;
    esac

# Uninstall binary and symlink
uninstall:
    cargo uninstall ticket-rs
    rm -f "{{cargo_bin}}/tk"
    @echo "Uninstalled ticket-rs"
    @echo "If you added shell completions, remove the '# ticket shell completions' block from your shell config."

# Remove build artifacts
clean:
    cargo clean
