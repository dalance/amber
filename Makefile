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
	cp target/x86_64-unknown-linux-gnu/release/ambs target/x86_64-unknown-linux-gnu/release/ambr
	zip amber-${VERSION}-x86_64-lnx.zip target/x86_64-unknown-linux-gnu/release/amb*

release_win64:
	cargo build --release --target=x86_64-pc-windows-gnu
	cp target/x86_64-pc-windows-gnu/release/ambs target/x86_64-pc-windows-gnu/release/ambr
	zip amber-${VERSION}-x86_64-win.zip target/x86_64-pc-windows-gnu/release/amb*

release_osx64:
	cargo build --release --target=x86_64-apple-darwin
	cp target/x86_64-apple-darwin/release/ambs target/x86_64-apple-darwin/release/ambr
	zip amber-${VERSION}-x86_64-osx.zip target/x86_64-apple-darwin/release/amb*

