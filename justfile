id := "com.coreparadox.opendeck.magtran-m3.sdPlugin"

default: check

release: bump package tag

package: build-linux collect zip

check:
    cargo clippy -- -D warnings
    cargo test

fmt:
    cargo fmt

bump next=`git cliff --bumped-version | tr -d "v"`:
    git diff --cached --exit-code

    @echo "We will bump version to {{next}}, press any key"
    read ans

    sed -i 's/"Version": ".*"/"Version": "{{next}}"/g' manifest.json
    sed -i 's/^version = ".*"$/version = "{{next}}"/g' Cargo.toml

tag next=`git cliff --bumped-version`:
    @echo "Generating changelog"
    git cliff -o CHANGELOG.md --tag {{next}}

    @echo "We will now commit the changes, please review before pressing any key"
    read ans

    git add .
    git commit -m "chore(release): {{next}}"
    git tag "{{next}}"

build-linux:
    cargo build --release --target x86_64-unknown-linux-gnu --target-dir target/plugin-linux

build-mac:
    docker run --rm -v $(pwd):/io -w /io ghcr.io/rust-cross/cargo-zigbuild:sha-eba2d7e cargo zigbuild --release --target universal2-apple-darwin --target-dir target/plugin-mac

build-win:
    docker run --rm -v $(pwd):/io -w /io rust:latest sh -c "apt-get update && apt-get install -y gcc-mingw-w64-x86-64 && rustup target add x86_64-pc-windows-gnu && cargo build --release --target x86_64-pc-windows-gnu --target-dir target/plugin-win"

clean:
    cargo clean
    rm -rf build

collect:
    rm -rf build
    mkdir -p build/{{id}}
    cp -r assets build/{{id}}
    cp manifest.json build/{{id}}
    cp target/plugin-linux/x86_64-unknown-linux-gnu/release/opendeck-m3 build/{{id}}/opendeck-m3-linux

[working-directory: "build"]
zip:
    zip -r opendeck-m3.plugin.zip {{id}}/
