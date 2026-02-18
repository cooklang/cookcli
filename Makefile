# Docker
IMAGE_NAME := $(shell yq '.services.cookcli.image' docker-compose.yml)

all: build

build: assets
	cargo build

css:
	npm run build-css

css-watch:
	npm run watch-css

js:
	npm run build-js

assets: css js

dev_assets:
	npm run watch-css & npm run watch-js

release: assets
	cargo build --release

dev_server: assets
	cargo run -- server ./seed --port 9080

dev: css-watch
	cargo run -- server ./seed

test:
	cargo test

clean:
	cargo clean
	rm -rf static/css/tailwind.css
	rm -rf static/js/editor.bundle.js

docker-build:
	docker build -t $(IMAGE_NAME) .

docker-push:
	docker push $(IMAGE_NAME)

docker: docker-build docker-push

.PHONY: all build release dev_server test clean css js assets dev_assets docker-build docker-push docker