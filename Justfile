default: build

build:
	RUSTFLAGS="-C target-cpu=native" cargo build

run:
	RUST_LOG=corna=debug,smithay_client_toolkit=warn WAYLAND_DEBUG=0 cargo run

restart:
	./restart.sh

fmt:
	cargo fmt

clippy:
	cargo clippy -- -D warnings

clean:
	cargo clean