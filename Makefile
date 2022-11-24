.PHONY: test
test:
	cargo test

.PHONY: clean
clean:
	cargo clean

.PHONY: build
build:
	cargo build --target wasm32-unknown-unknown

.PHONY: release
release:
	cargo build --release --target wasm32-unknown-unknown
