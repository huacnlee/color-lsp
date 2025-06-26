build:
	cargo build --release
	sudo rm /usr/local/bin/color-lsp
	sudo cp target/release/color-lsp /usr/local/bin/
