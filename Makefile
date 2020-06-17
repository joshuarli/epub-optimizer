name = epub-optimizer

target/debug/$(name): src/main.rs format lint
	cargo build
	ln -sf target/debug/$(name) $(name)

.PHONY: release
release: target/x86_64-unknown-linux-musl/release/$(name)
target/x86_64-unknown-linux-musl/release/$(name): src/main.rs format lint
	rustup target add --toolchain=stable x86_64-unknown-linux-musl 2> /dev/null
	cargo build --release --target x86_64-unknown-linux-musl
	strip --strip-all $@
	ln -sf $@ $(name)
	ls -lah $@
	file $@

.PHONY: lint
lint:
	@rustup component add clippy --toolchain stable 2> /dev/null
	@cargo +stable clippy --all-features --all --tests --examples -- -D clippy::all

.PHONY: format
format:
	@rustup component add rustfmt --toolchain stable 2> /dev/null
	cargo +stable fmt

.PHONY: clean
clean:
	rm -f target/debug/$(name) target/x86_64-unknown-linux-musl/$(name) $(name)
