VERSION=$(shell cat VERSION)
CURRENT_PATH=$(shell pwd)

all: release

release: prepare release_linux release_macos

prepare: update_version inject_seed inject_frontend
release_linux: build_linux archive_linux #smoketest_linux
release_macos: build_macos archive_macos

inject_seed:
	echo "Building frontend and injecting assets"
	bash ./scripts/inject_frontend.sh

inject_frontend:
	echo "Building frontend and injecting assets"
	bash ./scripts/inject_frontend.sh

update_version:
	echo "Setting version to $(VERSION)"
	sed -i '' -E "s|v[0-9]+.[0-9]+.[0-9]+|v$(VERSION)|g" Sources/CookCLI/Commands/Version.swift


build_linux:
	docker build -t cook-builder .
	docker run  --volume $(CURRENT_PATH):/src --workdir /src --entrypoint "swift" -it cook-builder build --configuration release -Xswiftc -static-stdlib

archive_linux:
	cd .build/x86_64-unknown-linux-gnu/release/ && zip "CookCLI_$(VERSION)_linux_amd64.zip" cook
	mv ".build/x86_64-unknown-linux-gnu/release/CookCLI_$(VERSION)_linux_amd64.zip" ./releases/

smoketest_linux:
	docker run -v $(CURRENT_PATH):/src -it ubuntu /src/.build/x86_64-unknown-linux-gnu/release/cook recipe read /src/samples/Borsch.cook


build_macos:
	swift build --configuration release  --static-swift-stdlib

archive_macos:
	cd .build/x86_64-apple-macosx/release/ && zip "CookCLI_$(VERSION)_darwin_amd64.zip" cook
	mv ".build/x86_64-apple-macosx/release/CookCLI_$(VERSION)_darwin_amd64.zip" ./releases/
