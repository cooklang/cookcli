
#Build linux
docker run -v $PWD:/src -it swift /bin/bash
swift build --configuration release -Xswiftc -static-stdlib -Xlinker -lCFURLSessionInterface -Xlinker -lcurl

#Build macos
swift build --configuration release  --static-swift-stdlib

# Archive
cd .build/x86_64-apple-macosx/release/
zip CookCLI_0.0.1_darwin_amd64.zip cook
cd -
mv .build/x86_64-apple-macosx/release/CookCLI_0.0.1_darwin_amd64.zip ./releases/


cd .build/x86_64-unknown-linux-gnu/release/
zip CookCLI_0.0.1_linux_amd64.zip cook
cd -
mv .build/x86_64-unknown-linux-gnu/release/CookCLI_0.0.1_linux_amd64.zip ./releases/

