id := "com.coreparadox.opendeck.magtran-m3.sdPlugin"

release-linux: clean check build-linux copy-assets package
release-mac: clean check build-mac copy-assets package
release-win: clean check build-win copy-assets package

release: clean check build-linux build-mac build-win copy-assets package

check:
    cargo clippy -- -D warnings
    cargo test

build-linux:
    cargo build --release --target x86_64-unknown-linux-gnu --target-dir target/plugin-linux

build-mac:
    docker run --rm -v $(pwd):/io -w /io ghcr.io/rust-cross/cargo-zigbuild:latest cargo zigbuild --release --target universal2-apple-darwin --target-dir target/plugin-mac

build-win:
    rustup target add x86_64-pc-windows-gnu
    cargo zigbuild --release --target x86_64-pc-windows-gnu --target-dir target/plugin-win

clean:
    cargo clean
    rm -rf build

copy-assets:
    rm -rf build/{{id}}
    mkdir -p build/{{id}}
    cp -r assets build/{{id}}
    cp manifest.json build/{{id}}
    cp target/plugin-linux/x86_64-unknown-linux-gnu/release/opendeck-magtran-m3 build/{{id}}/com.coreparadox.opendeck.magtran-m3-linux
    if [ -f target/plugin-mac/universal2-apple-darwin/release/opendeck-magtran-m3 ]; then cp target/plugin-mac/universal2-apple-darwin/release/opendeck-magtran-m3 build/{{id}}/com.coreparadox.opendeck.magtran-m3-mac; fi
    if [ -f target/plugin-win/x86_64-pc-windows-gnu/release/opendeck-magtran-m3.exe ]; then cp target/plugin-win/x86_64-pc-windows-gnu/release/opendeck-magtran-m3.exe build/{{id}}/com.coreparadox.opendeck.magtran-m3-win.exe; fi

[working-directory: "build"]
package:
    zip -r com.coreparadox.opendeck.magtran-m3.zip {{id}}/
