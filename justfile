id := "com.coreparadox.opendeck.magtran-m3.sdPlugin"

release: clean check build-linux copy-assets package

check:
    cargo clippy -- -D warnings
    cargo test

build-linux:
    cargo build --release --target x86_64-unknown-linux-gnu --target-dir target/plugin-linux

clean:
    cargo clean
    rm -rf build

copy-assets:
    rm -rf build/{{id}}
    mkdir -p build/{{id}}
    cp -r assets build/{{id}}
    cp manifest.json build/{{id}}
    cp target/plugin-linux/x86_64-unknown-linux-gnu/release/opendeck-magtran-m3 build/{{id}}/com.coreparadox.opendeck.magtran-m3-linux

[working-directory: "build"]
package:
    zip -r com.coreparadox.opendeck.magtran-m3.zip {{id}}/
