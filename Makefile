VERSION=$(shell cat VERSION)
CURRENT_PATH=$(shell pwd)

all: release

release: prepare release_linux release_macos

prepare: update_version inject_seed inject_frontend
release_linux: build_linux archive_linux #smoketest_linux
release_macos: build_macos archive_macos

inject_seed:
	echo "Inline sample recipes"
	bash ./scripts/inject_seed.sh

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
	docker run -v $(CURRENT_PATH):/src -it ubuntu /src/.build/x86_64-unknown-linux-gnu/release/cook recipe read /src/seed/Borsch.cook


build_macos:
	swift build --configuration release --arch arm64 --arch x86_64

check_env:
	if test "$(SIGNING_IDENTIFIER)" = "" ; then \
		echo "SIGNING_IDENTIFIER not set"; \
		exit 1; \
	fi

# You need to define `SIGNING_IDENTIFIER` environment variable. the value looks like "Developer ID Application: <TEAM NAME> (<TEAM_ID>)". You can see <TEAM NAME> and <TEAM_ID> at https://developer.apple.com/account/#!/membership
# Run `xcrun notarytool store-credentials` to store the passowrd
archive_macos: check_env
	rm -rf "./releases/CookCLI_$(VERSION)_darwin_amd64_arm64"
	rm -rf "./releases/CookCLI_$(VERSION)_darwin_amd64_arm64.zip"
	mkdir "./releases/CookCLI_$(VERSION)_darwin_amd64_arm64"
	cp .build/apple/Products/Release/cook "./releases/CookCLI_$(VERSION)_darwin_amd64_arm64"
	codesign --force --options runtime --deep-verify --verbose --sign "$(SIGNING_IDENTIFIER)" "./releases/CookCLI_$(VERSION)_darwin_amd64_arm64/cook"
	ditto -c -k "./releases/CookCLI_$(VERSION)_darwin_amd64_arm64" "./releases/CookCLI_$(VERSION)_darwin_amd64_arm64.zip"
	rm -rf "./releases/CookCLI_$(VERSION)_darwin_amd64_arm64"
	xcrun notarytool submit "./releases/CookCLI_$(VERSION)_darwin_amd64_arm64.zip" --keychain-profile 'AC_PASSWORD' --wait
