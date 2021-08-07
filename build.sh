
#Build linux
docker build -t cook-builder .
docker run  --volume $PWD:/src \
            --workdir /src \
            --entrypoint "swift" \
            -it cook-builder build --configuration release -Xswiftc -static-stdlib

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


# test linux
docker run -v $PWD:/src -it ubuntu /src/.build/x86_64-unknown-linux-gnu/release/cook recipe read /src/samples/Borsch.cook


swift build -Xlinker -L/usr/lib/x86_64-linux-gnu/mit-krb5 -Xlinker -lcurl -Xlinker -lnghttp2 -Xlinker -lidn2 -Xlinker -lrtmp -Xlinker -lpsl -Xlinker -lssl -Xlinker -lcrypto -Xlinker -lssl -Xlinker -lcrypto -Xlinker -Wl,-Bsymbolic-functions -Xlinker -Wl,-z,relro -Xlinker -lgssapi_krb5 -Xlinker -lkrb5 -Xlinker -lk5crypto -Xlinker -lcom_err -Xlinker -llber -Xlinker -lldap -Xlinker -llber -Xlinker -lz
