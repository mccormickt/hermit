CARGO=cargo +nightly 
BUILD_ARGS=--package hermit-wasm --target=wasm32-unknown-unknown

.PHONY: test
test:
	$(CARGO) test

.PHONY: clean
clean:
	cargo clean

.PHONY: build
build:
	$(CARGO) build $(BUILD_ARGS)

.PHONY: build-tcp
build:
	$(CARGO) build $(BUILD_ARGS) --no-default-features --features tcp

.PHONY: release
release:
	$(CARGO) build $(BUILD_ARGS) --release 

.PHONY: release-tcp
release:
	$(CARGO) build $(BUILD_ARGS) --release --no-default-features --features tcp

.PHONY: run
run: build
	cd hermit-wasm; \
	WASM_PATH="../target/wasm33-unknown-unknown/debug/hermit_wasm.wasm" docker compose up --build --remove-orphans | grep "\[wasm\]\|Starting"
