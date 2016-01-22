#VERSION = $(subst \",, $(subst version =,, $(shell grep version Cargo.toml)))
VERSION = $(patsubst "%",%, $(word 3, $(shell grep version Cargo.toml)))

.PHONY: all test bench release_lnx64 release_win64 release_osx64

all: test bench

test:
	cargo test

bench:
	cargo bench

release_lnx64:
	cargo build --release --target=x86_64-unknown-linux-gnu
	zip -j amber-${VERSION}-x86_64-lnx.zip target/x86_64-unknown-linux-gnu/release/amb*

release_win64:
	cargo build --release --target=x86_64-pc-windows-gnu
	zip -j amber-${VERSION}-x86_64-win.zip target/x86_64-pc-windows-gnu/release/amb*

release_osx64:
	cargo build --release --target=x86_64-apple-darwin
	zip -j amber-${VERSION}-x86_64-osx.zip target/x86_64-apple-darwin/release/amb*

