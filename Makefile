all: build

build: css
	cargo build

css:
	npm run build-css

css-watch:
	npm run watch-css

release: css
	cargo build --release

dev_server: css
	cargo run -- server ./seed

dev: css-watch
	cargo run -- server ./seed

test:
	cargo test

clean:
	cargo clean
	rm -rf static/css/tailwind.css

.PHONY: all build release dev_server test clean