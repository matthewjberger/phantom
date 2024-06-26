set windows-shell := ["powershell.exe"]

export RUST_LOG := "info"
export RUST_BACKTRACE := "1"

@just:
    just --list

build:
    cargo build -r

check:
    cargo check --all --tests
    cargo fmt --all --check

docs $project="phantom":
    cargo doc --open -p {{project}}

format:
    cargo fmt --all

fix:
    cargo clippy --all --tests --fix

lint:
    cargo clippy --all --tests -- -D warnings

run $project="editor":
    cargo run -r -p {{project}}
    
udeps:
    cargo machete

test:
    cargo test --all -- --nocapture

@versions:
    rustc --version
    cargo fmt -- --version
    cargo clippy -- --version

