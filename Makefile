.PHONY: test
test:
	cargo +nightly test

.PHONY: clean
clean:
	cargo clean

.PHONY: build
build:
	cargo +nightly build --target=wasm32-unknown-unknown

.PHONY: release
release:
	cargo +nightly build --release --target=wasm32-unknown-unknown

.PHONY: run
run: build
	WASM_PATH="./target/wasm32-unknown-unknown/debug/hermit.wasm" docker compose up --build --remove-orphans | grep "\[wasm\]\|Starting"
