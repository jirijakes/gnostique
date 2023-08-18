export RUST_BACKTRACE := "full"
export RUST_LOG := "none,gnostique=info,sqlx=info,hyper=info,relm4=warn"

@_default:
    just --list

sync:
    fossil update

upgrade:
    cargo upgrade --incompatible

build:
    cargo build

build--:
    watchexec -e rs -- just build

run:
    cargo run

run--:
    watchexec -e rs -- just run

refresh: sync upgrade build

status:
    fossil status --extra --changed --missing --deleted --added

diff path="":
    fossil diff {{path}}

test name="":
    cargo test {{name}} -- --nocapture

test-- name="":
    watchexec -e rs -- just test {{name}}

show:
    fossil timeline -n 1 --full -v

prepare:
    cargo +nightly fmt

[no-exit-message]
check:
    cargo +nightly fmt --check
