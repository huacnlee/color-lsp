build:
	cargo build --release
	sudo cp -f target/release/color-lsp /usr/local/bin/color-lsp
